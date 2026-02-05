use ratatui::layout::Rect;

pub mod events;
pub mod render;
pub mod state;

const MODAL_WIDTH_RATIO: f32 = 0.85;
const MODAL_HEIGHT_RATIO: f32 = 0.75;

/// Calculate the modal area centered within `full_area`.
/// Computes margins first with rounding, then applies symmetrically.
pub fn modal_area(full_area: Rect) -> Rect {
    let h_margin = ((full_area.width as f32 * (1.0 - MODAL_WIDTH_RATIO)) / 2.0).round() as u16;
    let v_margin = ((full_area.height as f32 * (1.0 - MODAL_HEIGHT_RATIO)) / 2.0).round() as u16;
    Rect::new(
        full_area.x + h_margin,
        full_area.y + v_margin,
        full_area.width.saturating_sub(2 * h_margin),
        full_area.height.saturating_sub(2 * v_margin),
    )
}
