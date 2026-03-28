use unicode_width::UnicodeWidthChar;

/// Display width of a string slice in terminal columns.
pub fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
        .sum()
}

pub fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut p = pos - 1;
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

pub fn next_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut p = pos + 1;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

/// Move cursor backward to the start of the previous word.
pub fn prev_word_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let chars: Vec<(usize, char)> = s[..pos].char_indices().collect();
    if chars.is_empty() {
        return 0;
    }
    let mut i = chars.len() - 1;
    while i > 0 && chars[i].1.is_whitespace() {
        i -= 1;
    }
    while i > 0 && !chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    chars[i].0
}

/// Move cursor forward past the next word.
pub fn next_word_boundary(s: &str, pos: usize) -> usize {
    let len = s.len();
    if pos >= len {
        return len;
    }
    let mut p = pos;
    for ch in s[p..].chars() {
        if !ch.is_whitespace() {
            break;
        }
        p += ch.len_utf8();
    }
    for ch in s[p..].chars() {
        if ch.is_whitespace() {
            break;
        }
        p += ch.len_utf8();
    }
    p
}

/// Find the byte offset of the start of the current hard line.
pub fn line_start(s: &str, pos: usize) -> usize {
    match s[..pos].rfind('\n') {
        Some(nl) => nl + 1,
        None => 0,
    }
}

// ── Word-level wrapping ─────────────────────────────────────────────────────

/// Compute visual line ranges `(start_byte, end_byte)` in the original text.
///
/// Hard newlines produce a gap between consecutive ranges (the `\n` byte).
/// Soft wraps produce adjacent ranges (end of one == start of next).
pub fn compute_visual_lines(text: &str, width: usize) -> Vec<(usize, usize)> {
    if text.is_empty() {
        return vec![(0, 0)];
    }
    if width == 0 {
        return vec![(0, text.len())];
    }

    let mut lines = Vec::new();
    let mut offset = 0usize;

    let parts: Vec<&str> = text.split('\n').collect();
    for (part_idx, hard_line) in parts.iter().enumerate() {
        let hard_start = offset;

        if hard_line.is_empty() {
            lines.push((hard_start, hard_start));
        } else {
            word_wrap_line(hard_line, hard_start, width, &mut lines);
        }

        offset += hard_line.len();
        if part_idx < parts.len() - 1 {
            offset += 1; // skip \n
        }
    }

    if lines.is_empty() {
        lines.push((0, 0));
    }
    lines
}

/// Word-wrap a single hard line, appending visual line ranges to `lines`.
fn word_wrap_line(line: &str, base_offset: usize, width: usize, lines: &mut Vec<(usize, usize)>) {
    let mut line_start = 0usize; // relative to `line`
    let mut col = 0usize;
    let mut last_break: Option<usize> = None; // byte pos (relative to `line`) after last whitespace

    for (i, ch) in line.char_indices() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(1);

        if col + cw > width && col > 0 {
            if let Some(bp) = last_break {
                if bp > line_start {
                    lines.push((base_offset + line_start, base_offset + bp));
                    line_start = bp;
                    col = line[bp..i]
                        .chars()
                        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
                        .sum();
                    last_break = None;
                } else {
                    lines.push((base_offset + line_start, base_offset + i));
                    line_start = i;
                    col = 0;
                    last_break = None;
                }
            } else {
                lines.push((base_offset + line_start, base_offset + i));
                line_start = i;
                col = 0;
            }
        }

        col += cw;

        if ch.is_whitespace() {
            last_break = Some(i + ch.len_utf8());
        }
    }

    lines.push((base_offset + line_start, base_offset + line.len()));
}

/// Produce a renderable string with newlines inserted at word-wrap points.
/// Render this with `Paragraph::new(...)` (no `.wrap()`).
pub fn wrap_text(text: &str, width: usize) -> String {
    let lines = compute_visual_lines(text, width);
    let mut result = String::with_capacity(text.len() + lines.len());
    for (i, &(start, end)) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        result.push_str(&text[start..end]);
    }
    result
}

/// Calculate cursor `(x, y)` in word-wrapped text.
/// Uses the same wrapping as `wrap_text` so they always agree.
pub fn cursor_position_in_wrapped(text: &str, byte_pos: usize, width: usize) -> (usize, usize) {
    let lines = compute_visual_lines(text, width);
    let byte_pos = byte_pos.min(text.len());

    for (y, &(start, end)) in lines.iter().enumerate() {
        let next_start = if y + 1 < lines.len() {
            lines[y + 1].0
        } else {
            text.len() + 1
        };

        if byte_pos < next_start || y == lines.len() - 1 {
            let clamped = byte_pos.min(end);
            let x: usize = text[start..clamped]
                .chars()
                .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
                .sum();
            return (x, y);
        }
    }

    (0, 0)
}
