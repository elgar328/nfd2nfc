use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Calculate the number of wrapped lines for a message given inner width (unicode-aware).
pub fn wrapped_line_count(text: &str, width: usize) -> usize {
    if width == 0 {
        return 1;
    }
    let text_width = UnicodeWidthStr::width(text);
    if text_width == 0 {
        return 1;
    }
    text_width.div_ceil(width)
}

/// Wrap text to fit within max_width, respecting Unicode character boundaries.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for c in text.chars() {
        let char_width = c.width().unwrap_or(0);

        if current_width + char_width > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }

        current_line.push(c);
        current_width += char_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Resolve a mouse click Y coordinate to a list item index.
///
/// Given the Y range of a visible list area (`list_start_y..list_end_y`),
/// the scroll `offset`, and total `count` of items, returns the index of
/// the clicked item, or `None` if the click is outside the list bounds.
pub fn clicked_list_index(
    y: u16,
    list_start_y: u16,
    list_end_y: u16,
    offset: usize,
    count: usize,
) -> Option<usize> {
    if y < list_start_y || y >= list_end_y {
        return None;
    }
    let idx = (y - list_start_y) as usize + offset;
    (idx < count).then_some(idx)
}
