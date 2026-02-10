use std::time::{Duration, Instant};

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

const DISPLAY_DURATION: Duration = Duration::from_secs(3);
const SLIDE_PX_PER_SEC: f64 = 80.0;
const MAX_TOASTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Success,
    Error,
}

impl ToastLevel {
    fn color(&self) -> Color {
        match self {
            ToastLevel::Success => Color::Green,
            ToastLevel::Error => Color::Red,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ToastPhase {
    Display,
    SlideOut { started_at: Instant },
}

struct Toast {
    message: String,
    level: ToastLevel,
    created_at: Instant,
    phase: ToastPhase,
}

pub struct ToastState {
    toasts: Vec<Toast>,
}

impl ToastState {
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    pub fn push(&mut self, message: String, level: ToastLevel) {
        if self.toasts.len() >= MAX_TOASTS {
            if let Some(toast) = self
                .toasts
                .iter_mut()
                .find(|t| matches!(t.phase, ToastPhase::Display))
            {
                toast.phase = ToastPhase::SlideOut {
                    started_at: Instant::now(),
                };
            }
        }
        self.toasts.push(Toast {
            message,
            level,
            created_at: Instant::now(),
            phase: ToastPhase::Display,
        });
    }

    pub fn tick(&mut self) {
        for toast in &mut self.toasts {
            if toast.phase == ToastPhase::Display && toast.created_at.elapsed() >= DISPLAY_DURATION
            {
                toast.phase = ToastPhase::SlideOut {
                    started_at: Instant::now(),
                };
            }
        }

        // Remove toasts that have slid off screen
        let cols = crossterm::terminal::size().map(|(c, _)| c).unwrap_or(80);
        let max_width = (cols as u32 * 40 / 100).max(20) as u16;
        self.toasts.retain(|toast| {
            if let ToastPhase::SlideOut { started_at } = toast.phase {
                slide_offset(started_at) <= max_width + 1
            } else {
                true
            }
        });
    }

    pub fn render(&self, f: &mut Frame, content_area: Rect) {
        if self.toasts.is_empty() {
            return;
        }

        let max_width = (content_area.width as u32 * 40 / 100).max(20) as u16;
        let inner_width = max_width.saturating_sub(2); // border left + right
        let mut current_y = content_area.y + 2;

        for toast in &self.toasts {
            // Calculate wrapped line count using unicode width
            let line_count = wrapped_line_count(&toast.message, inner_width as usize);
            let toast_height = 2 + line_count as u16; // top border + content lines + bottom border

            // Stop if toast would exceed content area
            if current_y + toast_height > content_area.y + content_area.height {
                break;
            }

            let offset = match toast.phase {
                ToastPhase::Display => 0,
                ToastPhase::SlideOut { started_at } => slide_offset(started_at),
            };

            let x = content_area
                .x
                .saturating_add(content_area.width)
                .saturating_sub(max_width)
                .saturating_sub(1)
                .saturating_add(offset);

            // Skip rendering if completely off-screen
            let right_edge = content_area.x + content_area.width;
            if x >= right_edge {
                current_y += toast_height;
                continue;
            }

            // Clip width so the rect never exceeds the frame buffer boundary
            let visible_width = max_width.min(right_edge.saturating_sub(x));
            let toast_rect = Rect::new(x, current_y, visible_width, toast_height);

            let color = toast.level.color();

            // Remove right border when partially clipped for a natural slide-out effect
            let clipped = visible_width < max_width;
            let borders = if clipped {
                Borders::TOP | Borders::BOTTOM | Borders::LEFT
            } else {
                Borders::ALL
            };

            let block = Block::default()
                .borders(borders)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(color));

            let paragraph = Paragraph::new(Line::from(toast.message.as_str()))
                .style(Style::default().fg(color).bg(Color::Black))
                .block(block)
                .wrap(Wrap { trim: true });

            f.render_widget(Clear, toast_rect);
            f.render_widget(paragraph, toast_rect);

            current_y += toast_height;
        }
    }
}

fn slide_offset(started_at: Instant) -> u16 {
    (started_at.elapsed().as_secs_f64() * SLIDE_PX_PER_SEC) as u16
}

/// Calculate the number of wrapped lines for a message given inner width.
fn wrapped_line_count(message: &str, width: usize) -> usize {
    if width == 0 {
        return 1;
    }
    let text_width = UnicodeWidthStr::width(message);
    if text_width == 0 {
        return 1;
    }
    text_width.div_ceil(width)
}
