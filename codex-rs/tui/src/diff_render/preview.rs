//! Bounds inline diff work before allocating wrapped rows or syntax spans.

use super::TAB_WIDTH;
use ratatui::buffer::Buffer;
use ratatui::buffer::CellWidth;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use unicode_width::UnicodeWidthChar;

pub(super) const PREVIEW_ROWS: usize = 12;
const MAX_PREVIEW_BYTES: usize = 64 * 1024;

/// Counts a UTF-8 source prefix within a pre-render character-width budget.
/// The caller also limits final viewport rows after styling and wrapping.
/// Include the start of an oversized line, stopping its scan at either budget.
/// The byte ceiling also bounds zero-width content that consumes no columns.
/// Lines must retain their terminators so the count can slice the source.
pub(super) fn visible_byte_count<'a>(
    lines: impl Iterator<Item = &'a str>,
    content_width: usize,
    max_rows: usize,
) -> usize {
    let content_width = content_width.max(/*other*/ 1);
    let mut remaining_rows = max_rows;
    let mut count = 0;
    for line in lines {
        if remaining_rows == 0 {
            break;
        }
        let mut rows = 1;
        let mut col = 0;
        let end = line.floor_char_boundary(MAX_PREVIEW_BYTES - count);
        let visible = &line[..end];
        for (offset, ch) in visible.trim_end_matches(['\r', '\n']).char_indices() {
            let width = ch.width().unwrap_or(if ch == '\t' { TAB_WIDTH } else { 0 });
            if col > 0 && col + width > content_width {
                rows += 1;
                col = 0;
            }
            if rows > remaining_rows {
                return count + offset;
            }
            col += width;
        }
        remaining_rows -= rows;
        count += visible.len();
        if end < line.len() {
            return count;
        }
    }
    count
}

/// Keep a partial line using the viewport renderer's own grapheme and word
/// wrapping. Only the remaining preview rows are allocated; full views bypass it.
pub(super) fn rendered_prefix(
    line: Line<'static>,
    width: u16,
    max_rows: u16,
) -> Vec<Line<'static>> {
    let style = line.style;
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, max_rows);
    let mut buffer = Buffer::empty(area);
    Paragraph::new(line)
        .wrap(Wrap { trim: false })
        .render(area, &mut buffer);
    buffer
        .content
        .chunks(usize::from(width.max(/*other*/ 1)))
        .map(|cells| {
            let end = cells
                .iter()
                .rposition(|cell| cell.symbol() != " ")
                .map_or(/*default*/ 0, |index| index + 1);
            let mut spans = Vec::new();
            let mut column = 0;
            while column < end {
                let cell = &cells[column];
                spans.push(Span::styled(cell.symbol().to_string(), cell.style()));
                // Buffer cells following a wide grapheme are continuation cells.
                column += usize::from(cell.cell_width().max(/*other*/ 1));
            }
            Line::from(spans).style(style)
        })
        .collect()
}
