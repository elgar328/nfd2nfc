use crate::tui::app::events::MouseState;
use crate::tui::dir_browser::SelectionKind;
use crate::tui::shortcuts::{
    gap, nav_arrows, render_centered_options, shortcut_bracketed, space, ShortcutBlock,
};
use crate::tui::styles::{
    active_value_style, inactive_italic_style, inactive_style, key_style, label_style,
    reverse_value_style,
};
use crate::tui::tabs::config::modal::state::AddModalState;
use crossterm::event::KeyCode;
use nfd2nfc_core::config::PathAction;
use nfd2nfc_core::utils::abbreviate_home;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_add_modal(
    modal: &mut AddModalState,
    f: &mut Frame,
    _area: Rect,
    mouse: &mut MouseState,
) {
    let full_area = f.area();

    // Dim the entire background including header
    f.render_widget(
        Block::default().style(Style::default().bg(Color::DarkGray)),
        full_area,
    );

    let modal_area = super::modal_area(full_area);

    // Clear the area behind the modal and set background style
    f.render_widget(Clear, modal_area);
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black).fg(Color::White)),
        modal_area,
    );

    // Build shortcuts for title_bottom
    let is_parent = modal.browser.selection_kind() == SelectionKind::Parent;

    let mut items: Vec<(Vec<Span>, Option<KeyCode>)> = vec![space()];
    if !is_parent {
        items.push(shortcut_bracketed("↵", "Add", KeyCode::Enter));
        items.push(gap());
    }
    items.extend(nav_arrows());
    items.extend(vec![
        gap(),
        shortcut_bracketed(".", "Hidden", KeyCode::Char('.')),
        gap(),
        shortcut_bracketed("⎋", "Cancel", KeyCode::Esc),
        space(),
    ]);

    let inner = ShortcutBlock::new(Line::from(Span::styled(
        " Add Path ",
        Style::default().fg(Color::White),
    )))
    .items(items)
    .render(f, modal_area, mouse);

    // Current selected path
    let current_path = abbreviate_home(
        &modal
            .browser
            .selected_entry()
            .filter(|e| !e.is_parent)
            .map(|e| e.path.to_string_lossy().to_string())
            .unwrap_or_else(|| modal.browser.current_dir.to_string_lossy().to_string()),
    );

    // Height: border(2) + path line(1)
    let path_box_height = 3u16;
    modal.path_box_height = path_box_height;

    let chunks = Layout::vertical([
        Constraint::Length(path_box_height), // Current path + description
        Constraint::Min(5),                  // File browser
        Constraint::Length(2),               // Options
    ])
    .split(inner);

    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Selected Path ");

    let path_para = Paragraph::new(Line::from(Span::styled(
        &current_path,
        Style::default().fg(Color::Cyan),
    )))
    .block(path_block)
    .wrap(Wrap { trim: false });

    f.render_widget(path_para, chunks[0]);

    // File browser list (directories only)
    let items: Vec<ListItem> = modal
        .browser
        .entries
        .iter()
        .filter(|e| e.is_dir)
        .map(|entry| {
            if entry.is_parent {
                ListItem::new(Line::from(vec![
                    Span::styled(" 📂", Style::default().fg(Color::Yellow)),
                    Span::styled("..", Style::default().fg(Color::Yellow)),
                ]))
            } else {
                let style = Style::default().fg(Color::White);
                ListItem::new(Line::from(vec![
                    Span::styled(" 📁", style),
                    Span::styled(&entry.name, style),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" Directories "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    // Adjust list state: map from entries index to filtered dir-only index
    let dir_indices = modal.browser.dir_indices();

    let mut adjusted_state = ratatui::widgets::ListState::default();
    *adjusted_state.offset_mut() = modal.browser.render_offset;
    if let Some(selected_entry_idx) = modal.browser.list_state.selected() {
        if let Some(pos) = dir_indices.iter().position(|&i| i == selected_entry_idx) {
            adjusted_state.select(Some(pos));
        }
    }

    f.render_stateful_widget(list, chunks[1], &mut adjusted_state);

    modal.browser.render_offset = adjusted_state.offset();

    // Options line
    let options_area = chunks[2];
    let na_style = inactive_italic_style();

    let option_items: Vec<(Vec<Span>, Option<KeyCode>)> = if is_parent {
        let action_spans = vec![
            Span::styled("Action: ", inactive_style()),
            Span::styled("N/A", na_style),
        ];
        let separator_spans = vec![Span::styled("  |  ", inactive_style())];
        let mode_spans = vec![
            Span::styled("Mode: ", inactive_style()),
            Span::styled("N/A", na_style),
        ];
        vec![
            (action_spans, None),
            (separator_spans, None),
            (mode_spans, None),
        ]
    } else {
        let is_ignore = modal.action == PathAction::Ignore;
        let action_text = if is_ignore { "Ignore" } else { "Watch" };
        let action_span = if is_ignore {
            Span::styled(action_text, reverse_value_style())
        } else {
            Span::styled(action_text, active_value_style())
        };

        let (mode_text, mode_style) = if is_ignore {
            ("Recursive", inactive_italic_style())
        } else {
            (modal.mode.as_str(), active_value_style())
        };

        let action_spans = vec![
            Span::styled("Ac", label_style()),
            Span::styled("t", key_style()),
            Span::styled("ion: ", label_style()),
            action_span,
        ];
        let separator_spans = vec![Span::raw("  |  ")];
        let mode_spans = vec![
            Span::styled("M", key_style()),
            Span::styled("ode: ", label_style()),
            Span::styled(mode_text, mode_style),
        ];

        vec![
            (action_spans, Some(KeyCode::Char('t'))),
            (separator_spans, None),
            (
                mode_spans,
                if !is_ignore {
                    Some(KeyCode::Char('m'))
                } else {
                    None
                },
            ),
        ]
    };

    render_centered_options(option_items, options_area, f, mouse);
}
