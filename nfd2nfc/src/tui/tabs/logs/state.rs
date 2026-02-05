use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use ratatui::style::{Color, Style};
use unicode_width::UnicodeWidthChar;

use crate::log_service::{self, LogEntry, LogEvent};

// ─────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────

const BOTTOM_PADDING_LINES: usize = 1;
pub(super) const MAX_LOG_ENTRIES: usize = 100_000;

// ─────────────────────────────────────────────────────────────
// LineCache
// ─────────────────────────────────────────────────────────────

/// A single cached wrapped line ready for rendering
pub struct CachedLine {
    pub text: String,
    pub style: Style,
    pub is_first: bool,
    pub display_time: String,
    pub entry_index: usize, // absolute index (monotonically increasing)
}

pub struct LineCache {
    pub lines: Vec<CachedLine>,
    width: usize,
    dirty: bool,
}

impl LineCache {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            width: 0,
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn needs_rebuild(&self, width: usize) -> bool {
        self.dirty || self.width != width
    }

    /// Full rebuild from all entries. Used on width change or initial load.
    pub fn rebuild(
        &mut self,
        entries: &VecDeque<LogEntry>,
        available_width: usize,
        base_index: usize,
    ) {
        self.lines.clear();

        for (i, entry) in entries.iter().enumerate() {
            let abs_index = base_index + i;
            let msg_style = entry_style(entry);

            let wrapped = wrap_text(&entry.message, available_width);
            for (j, text) in wrapped.into_iter().enumerate() {
                self.lines.push(CachedLine {
                    text,
                    style: msg_style,
                    is_first: j == 0,
                    display_time: if j == 0 {
                        entry.display_time.clone()
                    } else {
                        String::new()
                    },
                    entry_index: abs_index,
                });
            }
        }

        self.push_padding(base_index + entries.len());

        self.width = available_width;
        self.dirty = false;
    }

    /// Append lines for a single new entry (incremental update).
    /// Returns the number of lines added (excluding padding replacement).
    pub fn append_entry(
        &mut self,
        entry: &LogEntry,
        abs_index: usize,
        available_width: usize,
    ) -> usize {
        // Remove old bottom padding
        self.remove_padding();

        let msg_style = entry_style(entry);
        let wrapped = wrap_text(&entry.message, available_width);
        let line_count = wrapped.len();

        for (j, text) in wrapped.into_iter().enumerate() {
            self.lines.push(CachedLine {
                text,
                style: msg_style,
                is_first: j == 0,
                display_time: if j == 0 {
                    entry.display_time.clone()
                } else {
                    String::new()
                },
                entry_index: abs_index,
            });
        }

        // Re-add bottom padding
        self.push_padding(abs_index + 1);

        line_count
    }

    /// Remove cached lines belonging to the oldest entry (smallest entry_index).
    /// Returns the number of lines removed.
    pub fn remove_oldest_entry(&mut self) -> usize {
        if self.lines.is_empty() {
            return 0;
        }

        let oldest_index = self.lines[0].entry_index;
        let count = self
            .lines
            .iter()
            .take_while(|l| l.entry_index == oldest_index)
            .count();

        self.lines.drain(..count);
        count
    }

    /// Returns the line index of the first line belonging to the given entry_index
    pub fn first_line_of_entry(&self, entry_index: usize) -> usize {
        self.lines
            .iter()
            .position(|line| line.entry_index == entry_index && line.is_first)
            .unwrap_or(0)
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    fn remove_padding(&mut self) {
        // Padding lines have empty text, is_first=false, at the end
        while self
            .lines
            .last()
            .is_some_and(|l| l.text.is_empty() && !l.is_first)
        {
            self.lines.pop();
        }
    }

    fn push_padding(&mut self, padding_index: usize) {
        for _ in 0..BOTTOM_PADDING_LINES {
            self.lines.push(CachedLine {
                text: String::new(),
                style: Style::default(),
                is_first: false,
                display_time: String::new(),
                entry_index: padding_index,
            });
        }
    }
}

fn entry_style(entry: &LogEntry) -> Style {
    match entry.level.as_str() {
        "Fault" => Style::default().fg(Color::Red),
        "Error" => Style::default().fg(Color::Yellow),
        "Debug" | "Info" => Style::default().fg(Color::DarkGray),
        _ => Style::default(),
    }
}

// ─────────────────────────────────────────────────────────────
// wrap_text
// ─────────────────────────────────────────────────────────────

/// Wrap text to fit within max_width, respecting Unicode character boundaries
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for c in text.chars() {
        let char_width = c.width().unwrap_or(0);

        if current_width + char_width > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }

        current_line.push(c);
        current_width += char_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// ─────────────────────────────────────────────────────────────
// LogsState
// ─────────────────────────────────────────────────────────────

pub struct LogsState {
    pub entries: VecDeque<LogEntry>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub line_cache: LineCache,

    pub is_initial_loading: bool,
    pub visible_height: usize,

    /// Monotonically increasing counter for absolute entry indexing.
    /// The entry at entries[0] has absolute index = next_entry_index - entries.len().
    next_entry_index: usize,

    event_receiver: Receiver<LogEvent>,
}

impl std::fmt::Debug for LogsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogsState")
            .field("entries_len", &self.entries.len())
            .field("scroll_offset", &self.scroll_offset)
            .field("auto_scroll", &self.auto_scroll)
            .field("is_initial_loading", &self.is_initial_loading)
            .finish()
    }
}

impl LogsState {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel();

        // Initial load thread
        let load_tx = event_tx.clone();
        std::thread::spawn(move || {
            let entries = load_all_logs();
            let _ = load_tx.send(LogEvent::HistoryChunk { entries });
        });

        // Streaming thread
        let stream_tx = event_tx;
        std::thread::spawn(move || {
            log_service::stream_logs(stream_tx);
        });

        Self {
            entries: VecDeque::new(),
            scroll_offset: 0,
            auto_scroll: true,
            line_cache: LineCache::new(),
            is_initial_loading: true,
            visible_height: 0,
            next_entry_index: 0,
            event_receiver: event_rx,
        }
    }

    /// The absolute index of the first entry currently in the deque.
    pub fn base_index(&self) -> usize {
        self.next_entry_index - self.entries.len()
    }

    pub fn process_events(&mut self) {
        loop {
            match self.event_receiver.try_recv() {
                Ok(LogEvent::Live(entry)) => {
                    let abs_index = self.next_entry_index;
                    self.entries.push_back(entry);
                    self.next_entry_index += 1;

                    // Incremental cache update (only if cache is clean and width is known)
                    if !self.line_cache.dirty && self.line_cache.width > 0 {
                        self.line_cache.append_entry(
                            self.entries.back().unwrap(),
                            abs_index,
                            self.line_cache.width,
                        );
                    } else {
                        self.line_cache.mark_dirty();
                    }

                    // Evict oldest entries if over capacity
                    self.evict_overflow();

                    if self.auto_scroll {
                        self.scroll_offset = usize::MAX;
                    }
                }
                Ok(LogEvent::HistoryChunk { entries }) => {
                    self.merge_initial(entries);
                    self.is_initial_loading = false;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn merge_initial(&mut self, historical: Vec<LogEntry>) {
        let cutoff = historical.last().map(|e| &e.full_timestamp);
        let live_entries: Vec<LogEntry> = self
            .entries
            .drain(..)
            .filter(|e| cutoff.is_none_or(|ts| e.full_timestamp > *ts))
            .collect();

        self.entries = VecDeque::from(historical);
        self.entries.extend(live_entries);

        // Truncate to MAX_LOG_ENTRIES from the front if needed
        if self.entries.len() > MAX_LOG_ENTRIES {
            let excess = self.entries.len() - MAX_LOG_ENTRIES;
            self.entries.drain(..excess);
        }

        self.next_entry_index = self.entries.len();
        self.line_cache.mark_dirty();
        self.scroll_offset = usize::MAX;
    }

    /// Evict oldest entries when over MAX_LOG_ENTRIES.
    /// Adjusts scroll_offset and line cache incrementally.
    fn evict_overflow(&mut self) {
        while self.entries.len() > MAX_LOG_ENTRIES {
            self.entries.pop_front();

            // Remove corresponding lines from cache
            if !self.line_cache.dirty {
                let removed_lines = self.line_cache.remove_oldest_entry();

                // Adjust scroll offset
                if !self.auto_scroll {
                    if self.scroll_offset < removed_lines {
                        self.scroll_offset = 0;
                    } else {
                        self.scroll_offset -= removed_lines;
                    }
                }
            }
        }
    }

    pub fn go_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    pub fn go_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX;
        self.auto_scroll = true;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = self
            .line_cache
            .total_lines()
            .saturating_sub(self.visible_height);
        self.scroll_offset = self.scroll_offset.saturating_add(amount).min(max);
        if self.scroll_offset >= max {
            self.auto_scroll = true;
        }
    }

    pub fn is_loading(&self) -> bool {
        self.is_initial_loading
    }
}

// ─────────────────────────────────────────────────────────────
// Log loading
// ─────────────────────────────────────────────────────────────

fn load_all_logs() -> Vec<LogEntry> {
    log_service::get_log_history("365d").unwrap_or_default()
}
