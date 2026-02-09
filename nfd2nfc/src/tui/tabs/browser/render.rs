use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::tui::app::events::MouseState;
use crate::tui::app::render::content_area;
use crate::tui::component::SharedState;
use crate::tui::dir_browser::{SelectionKind, UnicodeForm};
use crate::tui::shortcuts::{gap, shortcut, shortcut_bracketed, space, ShortcutBlock};
use crate::tui::styles::{
    active_value_style, inactive_italic_style, inactive_style, key_style, label_style,
    reverse_value_style,
};
use crate::tui::tabs::browser::state::{BrowserAction, BrowserMode, BrowserState};
use nfd2nfc_core::utils::abbreviate_home;

pub fn render(
    state: &mut BrowserState,
    f: &mut Frame,
    area: Rect,
    _shared: &SharedState,
    mouse: &mut MouseState,
) {
    // Hide Convert button for Parent and ASCII file selections
    let kind = state.dir_browser.selection_kind();
    let hide_convert = matches!(kind, SelectionKind::Parent | SelectionKind::FileAscii);

    let mut items: Vec<(Vec<Span>, Option<KeyCode>)> = vec![space()];
    if !hide_convert {
        items.push(shortcut_bracketed("↵", "Convert", KeyCode::Enter));
        items.push(gap());
    }
    items.extend(vec![
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
        gap(),
        shortcut_bracketed(".", "Hidden", KeyCode::Char('.')),
        gap(),
        shortcut("Q", "uit", KeyCode::Char('q')),
        space(),
    ]);

    let inner = ShortcutBlock::new(Line::from(" Browser "))
        .items(items)
        .render(f, area, mouse);

    // Current path text
    let path_text = abbreviate_home(
        &state
            .dir_browser
            .effective_selected_entry()
            .filter(|e| !e.is_parent)
            .map(|e| e.path.to_string_lossy().to_string())
            .unwrap_or_else(|| state.dir_browser.current_dir.to_string_lossy().to_string()),
    );

    // Calculate dynamic path height based on text width and available inner width
    // inner.width - 2 accounts for the Path block's left/right borders
    let path_inner_width = inner.width.saturating_sub(2).max(1) as usize;
    let text_width = UnicodeWidthStr::width(path_text.as_str());
    let path_lines = ((text_width as f64 / path_inner_width as f64).ceil() as u16).clamp(1, 3);
    let path_height = path_lines + 2; // +2 for top/bottom borders
    state.path_height = path_height;

    let chunks = Layout::vertical([
        Constraint::Length(path_height), // Current path (dynamic)
        Constraint::Min(5),              // File list
        Constraint::Length(2),           // Options
    ])
    .split(inner);

    // Current path
    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Path ");
    let path_para = Paragraph::new(path_text)
        .block(path_block)
        .style(Style::default().fg(Color::Cyan))
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(path_para, chunks[0]);

    // File list
    let selected_idx = state.dir_browser.list_state.selected();

    let items: Vec<ListItem> = state
        .dir_browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            if entry.is_parent {
                ListItem::new(Line::from(vec![
                    Span::styled("📂", Style::default().fg(Color::Yellow)),
                    Span::styled("..", Style::default().fg(Color::Yellow)),
                ]))
            } else {
                let icon = if entry.is_dir { "📁" } else { "📄" };
                let style = if entry.is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };

                let mut spans = vec![
                    Span::styled(icon, style),
                    Span::styled(entry.name.clone(), style),
                ];

                if entry.form != UnicodeForm::ASCII {
                    let is_selected = selected_idx == Some(i);
                    let badge_style = if is_selected {
                        Style::default().fg(entry.form.color()).bg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::Black).bg(entry.form.color())
                    };
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("[{}]", entry.form.as_str()),
                        badge_style,
                    ));
                }

                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" Files "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut adjusted_state = state.dir_browser.list_state;
    *adjusted_state.offset_mut() = state.dir_browser.render_offset;

    f.render_stateful_widget(list, chunks[1], &mut adjusted_state);

    // Store rendered offset for mouse click calculations
    state.dir_browser.render_offset = adjusted_state.offset();

    // Bottom menu logic based on SelectionKind
    let selected_form = state.dir_browser.effective_selected_entry().map(|e| e.form);

    let gray = inactive_style();
    let gray_italic = inactive_italic_style();
    let active_key = key_style();
    let active_label = label_style();
    let active_text = active_value_style();
    let reverse_text = reverse_value_style();

    let (action_key_style, action_label_style, action_text, action_text_style) = match kind {
        // Inactive N/A
        SelectionKind::Parent | SelectionKind::FileAscii | SelectionKind::None => {
            (gray, gray, "N/A", gray_italic)
        }
        // File: auto-determined (gray)
        SelectionKind::FileNFD => (gray, gray, "Convert (NFD→NFC)", gray_italic),
        SelectionKind::FileNFC => (gray, gray, "Reverse (NFC→NFD)", gray_italic),
        // DirUnicode + NameOnly: auto-determined by dir name (gray)
        SelectionKind::DirUnicode if state.mode == BrowserMode::NameOnly => match selected_form {
            Some(UnicodeForm::NFD) => (gray, gray, "Convert (NFD→NFC)", gray_italic),
            _ => (gray, gray, "Reverse (NFC→NFD)", gray_italic),
        },
        // Directory (Recursive/Children): user-selectable (active)
        _ => match state.action {
            BrowserAction::Convert => (active_key, active_label, "Convert (NFD→NFC)", active_text),
            BrowserAction::Reverse => (active_key, active_label, "Reverse (NFC→NFD)", reverse_text),
        },
    };

    let (mode_key_style, mode_label_style, mode_text, mode_text_style) = match kind {
        SelectionKind::Parent | SelectionKind::FileAscii | SelectionKind::None => {
            (gray, gray, "N/A", gray_italic)
        }
        SelectionKind::FileNFD | SelectionKind::FileNFC => (gray, gray, "Name only", gray_italic),
        _ => (
            active_key,
            active_label,
            state.mode.as_str(),
            active_value_style(),
        ),
    };

    // Click areas: Action clickable for dirs except DirUnicode+NameOnly; Mode clickable for dirs only
    let action_clickable = kind.is_dir()
        && !(kind == SelectionKind::DirUnicode && state.mode == BrowserMode::NameOnly);
    let mode_clickable = kind.is_dir();

    // Register click areas for Action and Mode options (centered)
    let options_area = chunks[2];
    let action_spans = vec![
        Span::styled("Ac", action_label_style),
        Span::styled("t", action_key_style),
        Span::styled("ion: ", action_label_style),
        Span::styled(action_text, action_text_style),
    ];
    let separator_style = match kind {
        SelectionKind::Parent | SelectionKind::FileAscii | SelectionKind::None => gray,
        _ => Style::default(),
    };
    let separator_spans = vec![Span::styled("  |  ", separator_style)];
    let mode_spans = vec![
        Span::styled("M", mode_key_style),
        Span::styled("ode: ", mode_label_style),
        Span::styled(mode_text, mode_text_style),
    ];

    let option_items: Vec<(Vec<Span>, Option<KeyCode>)> = vec![
        (
            action_spans,
            if action_clickable {
                Some(KeyCode::Char('t'))
            } else {
                None
            },
        ),
        (separator_spans, None),
        (
            mode_spans,
            if mode_clickable {
                Some(KeyCode::Char('m'))
            } else {
                None
            },
        ),
    ];

    let total_width: u16 = option_items
        .iter()
        .flat_map(|(spans, _)| spans.iter())
        .map(|s| s.content.width() as u16)
        .sum();
    let x_start = options_area.x + (options_area.width.saturating_sub(total_width)) / 2;

    let option_spans = mouse.add_shortcuts(option_items, x_start, options_area.y);
    let options_para = Paragraph::new(Line::from(option_spans));
    let render_area = Rect::new(x_start, options_area.y, total_width, 1);
    f.render_widget(options_para, render_area);
}

// ─────────────────────────────────────────────────────────────
// Layout helper for mouse click calculations
// ─────────────────────────────────────────────────────────────

pub fn browser_list_y_range(path_height: u16) -> (u16, u16) {
    let ca = content_area();
    let inner_y = ca.y + 1;
    let list_start_y = inner_y + path_height + 1;
    let list_end_y = ca.y + ca.height - 1 - 2 - 1;
    (list_start_y, list_end_y)
}
