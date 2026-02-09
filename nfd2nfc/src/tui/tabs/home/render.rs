use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::tui::app::events::MouseState;
use crate::tui::component::SharedState;
use crate::tui::shortcuts::{gap, shortcut, shortcut_dimmed, space, ShortcutBlock};
use crate::tui::styles::{dimmed_style, watcher_status_span, StatusLabels};
use crate::tui::tabs::home::state::HomeState;

const HOME_CONTENT_WIDTH: u16 = 46;
const HOME_CONTENT_HEIGHT: u16 = 15; // 6+1+1+3+3+1

pub const ASCII_LOGO: &str = r#"o   o o--o o-o    --  o   o o--o   o-o
|\  | |    |  \  o  o |\  | |     /   
| \ | O-o  |   O   /  | \ | O-o  O    
|  \| |    |  /   /   |  \| |     \   
o   o o    o-o   o--o o   o o      o-o
"#;

pub fn render(
    _state: &mut HomeState,
    f: &mut Frame,
    area: Rect,
    shared: &SharedState,
    mouse: &mut MouseState,
) {
    // Build shortcuts for title_bottom (dimmed during pending operations)
    let items = if shared.async_op_pending {
        vec![
            space(),
            shortcut_dimmed("S", "tart/Stop"),
            gap(),
            shortcut_dimmed("R", "estart"),
            gap(),
            shortcut("Q", "uit", KeyCode::Char('q')),
            space(),
        ]
    } else if shared.watcher_running {
        vec![
            space(),
            shortcut("S", "top", KeyCode::Char('s')),
            gap(),
            shortcut("R", "estart", KeyCode::Char('r')),
            gap(),
            shortcut("Q", "uit", KeyCode::Char('q')),
            space(),
        ]
    } else {
        vec![
            space(),
            shortcut("S", "tart", KeyCode::Char('s')),
            gap(),
            shortcut("Q", "uit", KeyCode::Char('q')),
            space(),
        ]
    };

    let inner = ShortcutBlock::new(Line::from(" Home "))
        .items(items)
        .render(f, area, mouse);

    let centered = center_rect(inner, HOME_CONTENT_WIDTH, HOME_CONTENT_HEIGHT);

    let chunks = Layout::vertical([
        Constraint::Length(6), // Logo
        Constraint::Length(1), // Status
        Constraint::Length(1), // Separator line
        Constraint::Length(3), // Description (2 lines + margin)
        Constraint::Length(3), // Example box
        Constraint::Length(1), // Repository URL
    ])
    .split(centered);

    // Logo
    let logo = Paragraph::new(ASCII_LOGO)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(logo, chunks[0]);

    // Watcher status (shows pending operation if in progress)
    let status_text = watcher_status_span(
        shared.pending_operation,
        shared.watcher_running,
        &StatusLabels {
            pending_prefix: "  ",
            pending_suffix: "  ",
            running: "  Running  ",
            stopped: "  Stopped  ",
        },
    );

    let status_line = Line::from(vec![Span::raw("Watcher Status: "), status_text]);

    let status = Paragraph::new(status_line).alignment(Alignment::Center);
    f.render_widget(status, chunks[1]);

    // Separator line
    let separator = Paragraph::new("─".repeat(HOME_CONTENT_WIDTH as usize)).style(dimmed_style());
    f.render_widget(separator, chunks[2]);

    // Description text
    let description = Paragraph::new(
        "Automatically converts NFD filename to NFC\n\
         for seamless cross-platform compatibility.",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::White));
    f.render_widget(description, chunks[3]);

    // Example box
    let example_content = "ㅇㅅㄱㄹ.txt -> 일상기록.txt";
    let example = Paragraph::new(example_content)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(dimmed_style()),
        );
    f.render_widget(example, chunks[4]);

    // Repository URL (left) and version (right)
    let url = Paragraph::new("github.com/elgar328/nfd2nfc")
        .style(dimmed_style())
        .alignment(Alignment::Left);
    f.render_widget(url, chunks[5]);

    let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
        .style(dimmed_style())
        .alignment(Alignment::Right);
    f.render_widget(version, chunks[5]);
}

fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
