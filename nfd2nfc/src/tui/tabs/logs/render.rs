use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

/// Timestamp column width: "01-21 11:23:45" (14) + "  " (2)
const TIMESTAMP_COL_WIDTH: usize = 16;

use crate::tui::app::events::MouseState;
use crate::tui::component::SharedState;
use crate::tui::shortcuts::{ShortcutBlock, gap, shortcut, space};
use crate::tui::styles::{key_style, label_style};
use crate::tui::tabs::logs::state::{LogsState, MAX_LOG_ENTRIES};

fn format_count(n: usize) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

fn format_unit(n: usize, threshold: usize, frac_divisor: usize, suffix: &str) -> String {
    let whole = n / threshold;
    let frac = (n % threshold) / frac_divisor;
    if frac == 0 {
        format!("{}{}", whole, suffix)
    } else {
        format!("{}.{}{}", whole, frac, suffix)
    }
}

fn format_compact(n: usize) -> String {
    if n >= 1_000_000 {
        format_unit(n, 1_000_000, 100_000, "M")
    } else if n >= 1_000 {
        format_unit(n, 1_000, 100, "K")
    } else {
        format_count(n)
    }
}

pub fn render(
    state: &mut LogsState,
    f: &mut Frame,
    area: Rect,
    _shared: &SharedState,
    mouse: &mut MouseState,
) {
    let items: Vec<(Vec<Span>, Option<KeyCode>)> = vec![
        space(),
        (vec![Span::styled("[", label_style())], None),
        (vec![Span::styled("↑", key_style())], Some(KeyCode::Up)),
        (vec![Span::styled("↓", key_style())], Some(KeyCode::Down)),
        (
            vec![
                Span::styled("]", label_style()),
                Span::styled("Scroll", label_style()),
            ],
            None,
        ),
        gap(),
        (
            vec![
                Span::styled("Page", label_style()),
                Span::styled("U", key_style()),
                Span::styled("p", label_style()),
            ],
            Some(KeyCode::Char('u')),
        ),
        (vec![Span::styled("/", label_style())], None),
        (
            vec![
                Span::styled("D", key_style()),
                Span::styled("own", label_style()),
            ],
            Some(KeyCode::Char('d')),
        ),
        gap(),
        shortcut("T", "op", KeyCode::Char('t')),
        (vec![Span::styled("/", label_style())], None),
        shortcut("B", "ottom", KeyCode::Char('b')),
        gap(),
        shortcut("Q", "uit", KeyCode::Char('q')),
        space(),
    ];

    let count_label = format!(
        "{}/{} ",
        format_count(state.entries.len()),
        format_compact(MAX_LOG_ENTRIES)
    );
    let title = Line::from(vec![
        Span::raw(" Logs "),
        Span::styled(count_label, Style::default().fg(Color::DarkGray)),
    ]);

    let inner = ShortcutBlock::new(title)
        .items(items)
        .render(f, area, mouse);

    let logs_area = inner;

    state.visible_height = logs_area.height as usize;

    // Show loading state during initial load
    if state.is_loading() {
        let centered_area = Rect {
            y: logs_area.y + logs_area.height / 2,
            height: 1,
            ..logs_area
        };
        let loading = Paragraph::new("Loading logs...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(loading, centered_area);
        return;
    }

    if state.entries.is_empty() {
        let empty = Paragraph::new("No logs available")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(empty, logs_area);
        return;
    }

    let available_width = (logs_area.width as usize).saturating_sub(TIMESTAMP_COL_WIDTH + 1);

    // Rebuild cache if needed, preserving scroll position anchor
    if state.line_cache.needs_rebuild(available_width) {
        // Remember which entry is at the top of the viewport before rebuild
        let anchor_entry = if !state.auto_scroll {
            let clamped = state
                .scroll_offset
                .min(state.line_cache.total_lines().saturating_sub(1));
            state.line_cache.lines.get(clamped).map(|l| l.entry_index)
        } else {
            None
        };

        state
            .line_cache
            .rebuild(&state.entries, available_width, state.base_index());

        // Restore scroll position from anchor
        if let Some(entry_idx) = anchor_entry {
            state.scroll_offset = state.line_cache.first_line_of_entry(entry_idx);
        }
    }

    // Clamp scroll_offset
    let total = state.line_cache.total_lines();
    let max_offset = total.saturating_sub(state.visible_height);
    state.scroll_offset = state.scroll_offset.min(max_offset);

    // Render only the visible slice
    let end = (state.scroll_offset + state.visible_height).min(total);
    let visible_lines: Vec<Line> = state.line_cache.lines[state.scroll_offset..end]
        .iter()
        .map(|cached| {
            if cached.is_first {
                Line::from(vec![
                    Span::styled(
                        cached.display_time.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("  "),
                    Span::styled(cached.text.clone(), cached.style),
                ])
            } else {
                Line::from(vec![
                    Span::raw(" ".repeat(TIMESTAMP_COL_WIDTH)),
                    Span::styled(cached.text.clone(), cached.style),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, logs_area);

    // Scrollbar overlay on the right border
    if max_offset > 0 {
        let mut scrollbar_state = ScrollbarState::new(max_offset).position(state.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("┃");
        let scrollbar_area = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(2),
            ..area
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
