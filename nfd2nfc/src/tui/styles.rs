use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::tui::app::state::PendingWatcherOperation;

pub fn key_style() -> Style {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
}

pub fn label_style() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn dimmed_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn status_running_style() -> Style {
    Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD)
}

pub fn status_stopped_style() -> Style {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
}

pub fn status_pending_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn active_value_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn reverse_value_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn inactive_style() -> Style {
    Style::default().fg(Color::Gray)
}

pub fn inactive_italic_style() -> Style {
    Style::default()
        .fg(Color::Gray)
        .add_modifier(Modifier::ITALIC)
}

pub fn border_style() -> Style {
    Style::default().fg(Color::Gray)
}

/// Labels for watcher status display.
pub struct StatusLabels {
    pub pending_prefix: &'static str,
    pub pending_suffix: &'static str,
    pub running: &'static str,
    pub stopped: &'static str,
}

/// Build a styled Span for the current watcher status.
pub fn watcher_status_span(
    pending_op: Option<PendingWatcherOperation>,
    watcher_running: bool,
    labels: &StatusLabels,
) -> Span<'static> {
    if let Some(op) = pending_op {
        let label = match op {
            PendingWatcherOperation::Starting => {
                format!(
                    "{}Starting...{}",
                    labels.pending_prefix, labels.pending_suffix
                )
            }
            PendingWatcherOperation::Stopping => {
                format!(
                    "{}Stopping...{}",
                    labels.pending_prefix, labels.pending_suffix
                )
            }
            PendingWatcherOperation::Restarting => {
                format!(
                    "{}Restarting...{}",
                    labels.pending_prefix, labels.pending_suffix
                )
            }
        };
        Span::styled(label, status_pending_style())
    } else if watcher_running {
        Span::styled(labels.running, status_running_style())
    } else {
        Span::styled(labels.stopped, status_stopped_style())
    }
}
