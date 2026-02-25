use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use strum::IntoEnumIterator;

use crate::tui::app::state::App;
use crate::tui::component::TabComponent;
use crate::tui::styles::{StatusLabels, bold_fg, border_style, key_style, watcher_status_span};
use crate::tui::tabs::Tab;

pub struct AppLayout {
    pub header: Rect,
    pub content: Rect,
}

pub fn layout(area: Rect) -> AppLayout {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    AppLayout {
        header: chunks[0],
        content: chunks[1],
    }
}

pub fn content_area() -> Rect {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    layout(Rect::new(0, 0, cols, rows)).content
}

pub fn draw(f: &mut Frame, app: &mut App) {
    // Set default style for entire screen (black background, white foreground)
    let area = f.area();
    f.buffer_mut()
        .set_style(area, Style::default().bg(Color::Black).fg(Color::White));

    // Clear all click areas before rendering (each component will populate them)
    app.mouse_state.clear();

    draw_header(f, app);
    draw_content(f, app);

    let content_area = layout(f.area()).content;
    app.toast.render(f, content_area);
}

fn draw_header(f: &mut Frame, app: &mut App) {
    let area = layout(f.area()).header;
    let current_idx = app.current_tab.index();
    let divider = " │ ";

    // Status indicator (shows pending operation if in progress)
    let status = Line::from(watcher_status_span(
        app.async_operation.as_ref().map(|op| op.kind),
        app.watcher_running,
        &StatusLabels {
            pending_prefix: " ◐ ",
            pending_suffix: " ",
            running: " ● Running ",
            stopped: " ○ Stopped ",
        },
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style())
        .title(Span::styled(" NFD2NFC ", Style::default().fg(Color::White)))
        .title(status.right_aligned());

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build tab items — add_shortcuts registers click areas and returns spans
    let mut items: Vec<(Vec<Span>, Option<crossterm::event::KeyCode>)> = Vec::new();
    // Leading space to match original Tabs widget padding
    items.push((vec![Span::raw(" ")], None));
    items.extend(Tab::iter().enumerate().flat_map(|(i, tab)| {
        let superscript_style = key_style();
        let title_style = if tab.index() == current_idx {
            bold_fg(Color::LightCyan)
        } else {
            Style::default().fg(Color::White)
        };
        let mut result = Vec::new();
        if i > 0 {
            result.push((
                vec![Span::styled(divider, Style::default().fg(Color::Gray))],
                None,
            ));
        }
        result.push((
            vec![
                Span::styled(tab.superscript(), superscript_style),
                Span::styled(tab.title(), title_style),
            ],
            Some(tab.key()),
        ));
        result
    }));

    let spans = app.mouse_state.add_shortcuts(items, inner.x, inner.y);

    // Render tabs directly using the same spans used for click areas
    let tabs_line = Paragraph::new(Line::from(spans));
    f.render_widget(tabs_line, Rect::new(inner.x, inner.y, inner.width, 1));
}

fn draw_content(f: &mut Frame, app: &mut App) {
    let area = layout(f.area()).content;
    let shared = app.shared_state();
    match app.current_tab {
        Tab::Home => app.home.render(f, area, &shared, &mut app.mouse_state),
        Tab::Config => app.config.render(f, area, &shared, &mut app.mouse_state),
        Tab::Logs => app.logs.render(f, area, &shared, &mut app.mouse_state),
        Tab::Browser => app.browser.render(f, area, &shared, &mut app.mouse_state),
    }
}
