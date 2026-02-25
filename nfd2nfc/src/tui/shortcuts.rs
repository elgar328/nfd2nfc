use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::tui::app::events::MouseState;
use crate::tui::styles::{border_style, dimmed_style, key_style, label_style};

/// Builder for rendering a block with a bottom shortcut bar.
pub struct ShortcutBlock<'a> {
    title: Line<'a>,
    items: Vec<(Vec<Span<'a>>, Option<KeyCode>)>,
}

impl<'a> ShortcutBlock<'a> {
    pub fn new(title: Line<'a>) -> Self {
        Self {
            title,
            items: Vec::new(),
        }
    }

    pub fn items(mut self, items: Vec<(Vec<Span<'a>>, Option<KeyCode>)>) -> Self {
        self.items = items;
        self
    }

    /// Render the block with shortcuts and return the inner area for content.
    pub fn render(self, f: &mut Frame, area: Rect, mouse: &mut MouseState) -> Rect {
        let bottom_y = area.y + area.height - 1;
        let base_x = area.x + 1;

        let shortcut_spans = mouse.add_shortcuts(self.items, base_x, bottom_y);
        let shortcuts = Line::from(shortcut_spans);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title(self.title)
            .title_alignment(Alignment::Center)
            .title_bottom(shortcuts.left_aligned());

        let inner = block.inner(area);
        f.render_widget(block, area);
        inner
    }
}

// ─────────────────────────────────────────────────────────────
// Shortcut item helpers
// ─────────────────────────────────────────────────────────────

/// Active shortcut: key highlighted in red, label in cyan.
pub fn shortcut<'a>(
    key: &'a str,
    label: &'a str,
    code: KeyCode,
) -> (Vec<Span<'a>>, Option<KeyCode>) {
    (
        vec![
            Span::styled(key, key_style()),
            Span::styled(label, label_style()),
        ],
        Some(code),
    )
}

/// Dimmed shortcut: both key and label rendered in dimmed style, no click area.
pub fn shortcut_dimmed<'a>(key: &'a str, label: &'a str) -> (Vec<Span<'a>>, Option<KeyCode>) {
    (
        vec![
            Span::styled(key, dimmed_style()),
            Span::styled(label, dimmed_style()),
        ],
        None,
    )
}

/// Bracketed shortcut: `[key]label` with key highlighted, label in cyan.
pub fn shortcut_bracketed<'b>(
    key: &'b str,
    label: &'b str,
    code: KeyCode,
) -> (Vec<Span<'b>>, Option<KeyCode>) {
    (
        vec![
            Span::styled("[", label_style()),
            Span::styled(key, key_style()),
            Span::styled("]", label_style()),
            Span::styled(label, label_style()),
        ],
        Some(code),
    )
}

/// Navigation arrow keys: `[←↑↓→]Navigate`
pub fn nav_arrows() -> Vec<(Vec<Span<'static>>, Option<KeyCode>)> {
    vec![
        (vec![Span::styled("[", label_style())], None),
        (vec![Span::styled("←", key_style())], Some(KeyCode::Left)),
        (vec![Span::styled("↑", key_style())], Some(KeyCode::Up)),
        (vec![Span::styled("↓", key_style())], Some(KeyCode::Down)),
        (vec![Span::styled("→", key_style())], Some(KeyCode::Right)),
        (
            vec![
                Span::styled("]", label_style()),
                Span::styled("Navigate", label_style()),
            ],
            None,
        ),
    ]
}

/// Render option items centered horizontally within the given area.
pub fn render_centered_options(
    items: Vec<(Vec<Span>, Option<KeyCode>)>,
    area: Rect,
    f: &mut Frame,
    mouse: &mut MouseState,
) {
    let total_width: u16 = items
        .iter()
        .flat_map(|(spans, _)| spans.iter())
        .map(|s| s.content.width() as u16)
        .sum();
    let x_start = area.x + (area.width.saturating_sub(total_width)) / 2;
    let spans = mouse.add_shortcuts(items, x_start, area.y);
    let para = Paragraph::new(Line::from(spans));
    f.render_widget(para, Rect::new(x_start, area.y, total_width, 1));
}

/// Single space separator.
pub fn space() -> (Vec<Span<'static>>, Option<KeyCode>) {
    (vec![Span::raw(" ")], None)
}

/// Double space gap.
pub fn gap() -> (Vec<Span<'static>>, Option<KeyCode>) {
    (vec![Span::raw("  ")], None)
}
