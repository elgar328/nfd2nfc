use crossterm::event::KeyCode;
use ratatui::{
    layout::{Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

const CONFIG_TABLE_WIDTHS: [ratatui::layout::Constraint; 5] = [
    ratatui::layout::Constraint::Length(3),
    ratatui::layout::Constraint::Min(20),
    ratatui::layout::Constraint::Length(8),
    ratatui::layout::Constraint::Length(14),
    ratatui::layout::Constraint::Length(12),
];

use crate::tui::app::events::MouseState;
use crate::tui::component::SharedState;
use crate::tui::shortcuts::{gap, space, ShortcutBlock};
use crate::tui::styles::{key_style, label_style};
use crate::tui::tabs::config::modal::render::render_add_modal;
use crate::tui::tabs::config::state::ConfigState;
use nfd2nfc_core::config::{PathAction, PathEntry, PathMode, PathStatus};
use nfd2nfc_core::utils::abbreviate_home;

fn path_description(idx: usize, entries: &[PathEntry]) -> String {
    let entry = &entries[idx];
    let desc = match &entry.status {
        PathStatus::Active => {
            let desc = match (entry.action, entry.mode) {
                (PathAction::Watch, PathMode::Recursive) => {
                    "Watches for changes and auto-converts NFD names to NFC in this directory and all subdirectories."
                }
                (PathAction::Watch, PathMode::Children) => {
                    "Watches for changes and auto-converts NFD names to NFC in this directory only, not subdirectories."
                }
                (PathAction::Ignore, _) => "Excludes this directory and all subdirectories from watching.",
            };
            if let Some(p) = entry.overrides {
                format!("Overrides #{}. {}", p + 1, desc)
            } else {
                desc.to_string()
            }
        }
        PathStatus::Redundant(n) => {
            format!("Redundant: same action as #{}.", n + 1)
        }
        PathStatus::NotFound => "Path not found: this rule is not applied.".to_string(),
        PathStatus::NotADirectory => "Not a directory: this rule is not applied.".to_string(),
        PathStatus::PermissionDenied => "Permission denied: this rule is not applied.".to_string(),
    };
    format!("{}\n{}", entry.raw, desc)
}

pub fn render(
    state: &mut ConfigState,
    f: &mut Frame,
    area: Rect,
    _shared: &SharedState,
    mouse: &mut MouseState,
) {
    // Build title
    let title = if state.has_changes {
        Line::from(vec![
            Span::raw(" Config "),
            Span::styled(
                "[unsaved]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(" Config ")
    };

    // Build shortcuts (register click areas only when modal is not shown)
    let register = !state.modal.show;
    let reg = |code: KeyCode| if register { Some(code) } else { None };

    let mut items: Vec<(Vec<Span>, Option<KeyCode>)> = vec![
        space(),
        (
            vec![
                Span::styled("A", key_style()),
                Span::styled("dd", label_style()),
            ],
            reg(KeyCode::Char('a')),
        ),
        gap(),
        (
            vec![
                Span::styled("D", key_style()),
                Span::styled("elete", label_style()),
            ],
            reg(KeyCode::Char('d')),
        ),
        gap(),
        (
            vec![
                Span::styled("S", label_style()),
                Span::styled("o", key_style()),
                Span::styled("rt", label_style()),
            ],
            reg(KeyCode::Char('o')),
        ),
        gap(),
        (vec![Span::styled("[", label_style())], None),
        (
            vec![Span::styled("+", key_style())],
            reg(KeyCode::Char('+')),
        ),
        (
            vec![Span::styled("-", key_style())],
            reg(KeyCode::Char('-')),
        ),
        (
            vec![
                Span::styled("]", label_style()),
                Span::styled("Move", label_style()),
            ],
            None,
        ),
        gap(),
    ];

    if state.has_changes {
        items.push((
            vec![
                Span::styled("S", key_style()),
                Span::styled("ave", label_style()),
            ],
            reg(KeyCode::Char('s')),
        ));
        items.push(gap());
        items.push((
            vec![
                Span::styled("[", label_style()),
                Span::styled("⎋", key_style()),
                Span::styled("]", label_style()),
                Span::styled("Cancel", label_style()),
            ],
            reg(KeyCode::Esc),
        ));
        items.push(gap());
    }

    items.push((
        vec![
            Span::styled("Q", key_style()),
            Span::styled("uit", label_style()),
        ],
        reg(KeyCode::Char('q')),
    ));
    items.push(space());

    let inner = ShortcutBlock::new(title)
        .items(items)
        .render(f, area, mouse);

    // Determine if we need a description box
    let selected_idx = state.table_state.selected();
    let description = selected_idx.and_then(|idx| {
        if idx < state.config.paths.len() {
            Some(path_description(idx, &state.config.paths))
        } else {
            None
        }
    });

    let (table_area, desc_area) = if let Some(ref desc_text) = description {
        // Calculate how many lines the description will take when wrapped
        let inner_width = (if inner.width > 2 { inner.width - 2 } else { 1 }) as usize;
        let line_count = if inner_width > 0 {
            desc_text
                .lines()
                .map(|line| line.width().div_ceil(inner_width).max(1) as u16)
                .sum::<u16>()
                .max(1)
        } else {
            1u16
        };
        let box_height = line_count + 2; // borders
        let box_height = box_height.min(inner.height.saturating_sub(3)); // leave room for table

        if box_height >= 3 && inner.height > box_height + 2 {
            let chunks = Layout::vertical([
                ratatui::layout::Constraint::Min(0),
                ratatui::layout::Constraint::Length(box_height),
            ])
            .split(inner);
            (chunks[0], Some(chunks[1]))
        } else {
            (inner, None)
        }
    } else {
        (inner, None)
    };

    // Create table rows
    let rows: Vec<Row> = state
        .config
        .paths
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let is_active = matches!(entry.status, PathStatus::Active);

            let path_style = if is_active {
                Style::default()
            } else {
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::CROSSED_OUT)
            };

            let (mode_text, mode_style) = if entry.action == PathAction::Ignore {
                (
                    "Recursive",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                )
            } else {
                (entry.mode.as_str(), Style::default())
            };

            let abbreviated = abbreviate_home(&entry.raw);
            let path_cell = if is_active {
                Cell::from(abbreviated.as_str().to_owned())
            } else {
                Cell::from(Line::from(Span::styled(abbreviated, path_style)))
            };

            Row::new(vec![
                Cell::from(format!("{}", idx + 1)),
                path_cell,
                Cell::from(entry.action.as_str()),
                Cell::from(mode_text).style(mode_style),
                Cell::from(format!(
                    "{} {}",
                    entry.status.symbol(),
                    entry.status.as_str()
                )),
            ])
        })
        .collect();

    let widths = CONFIG_TABLE_WIDTHS;

    let header_cells = vec![
        Cell::from("#").style(Style::default().fg(Color::Cyan)),
        Cell::from("Path").style(Style::default().fg(Color::Cyan)),
        Cell::from(Line::from(vec![
            Span::styled("Ac", Style::default().fg(Color::Cyan)),
            Span::styled(
                "t",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("ion", Style::default().fg(Color::Cyan)),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                "M",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("ode", Style::default().fg(Color::Cyan)),
        ])),
        Cell::from("Status").style(Style::default().fg(Color::Cyan)),
    ];

    let table = Table::new(rows, widths)
        .header(Row::new(header_cells).bottom_margin(1))
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, table_area, &mut state.table_state);

    // Register table header click areas for Action and Mode columns
    if register {
        let col_rects = Layout::horizontal(widths).spacing(1).split(table_area);
        let header_y = table_area.y;
        // Action column (index 2) → 't' key (text "Action" = 6 chars)
        let action_rect = Rect::new(col_rects[2].x, header_y, 6, 1);
        mouse.add(action_rect, KeyCode::Char('t'));
        // Mode column (index 3) → 'm' key (text "Mode" = 4 chars)
        let mode_rect = Rect::new(col_rects[3].x, header_y, 4, 1);
        mouse.add(mode_rect, KeyCode::Char('m'));
    }

    // Render description box if selected
    if let (Some(desc_text), Some(da)) = (description, desc_area) {
        let desc_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Black));
        let desc_paragraph = Paragraph::new(desc_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: false })
            .block(desc_block);
        f.render_widget(desc_paragraph, da);
    }

    // Render add modal if active
    if state.modal.show {
        render_add_modal(&mut state.modal, f, area, mouse);
    }
}
