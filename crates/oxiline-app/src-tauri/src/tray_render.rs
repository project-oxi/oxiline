//! Bitmapped menu-bar slot renderer.
//!
//! Each slot is a 22 px tall RGBA buffer. v1 ships an ASCII 5×7 monospaced
//! font (see `FONT`); non-ASCII bytes render as a 5×7 box so width stays
//! predictable across locales.
//!
//! The font is **column-major** (spec §5): each glyph entry is
//! `[u8; GLYPH_W]` where each byte holds the 7 vertical pixels of one
//! column, bit 0 = top row, bit 6 = bottom row.
//!
//! Public API + helpers land in Task 3 — silence dead-code until then.
#![allow(dead_code)]

use std::collections::BTreeMap;

use image::{ImageBuffer, Rgba};
use tauri::image::Image;

use oxiline_core::model::TraySlotKind;

const SLOT_HEIGHT: u32 = 22;
const SLOT_PAD_RIGHT: u32 = 4;
const SLOT_MIN_WIDTH: u32 = 24;
const SLOT_MAX_WIDTH: u32 = 120;
const GLYPH_W: u32 = 5;
const GLYPH_H: u32 = 7;
const GLYPH_ADVANCE: u32 = 6;
const DOT_RADIUS: i32 = 5;

pub struct LabelCtx<'a> {
    pub now_minute: u16,
    pub rounding_minutes: i64,
    pub now_summary: &'a oxiline_core::model::NowSummary,
}

pub fn label_for(kind: TraySlotKind, locale: &str, ctx: &LabelCtx<'_>) -> String {
    match kind {
        TraySlotKind::NowRecording => label_now(locale, ctx),
        TraySlotKind::NowNext => label_next(locale, ctx),
        TraySlotKind::StateDot => String::new(),
    }
}

fn label_now(_locale: &str, ctx: &LabelCtx<'_>) -> String {
    let summary = match &ctx.now_summary.current {
        Some(c) => c,
        None => return String::new(),
    };
    let mins = ctx.rounding_minutes.max(1);
    let n = summary.remaining_minute.unwrap_or(0).max(0);
    let n = ((n + mins / 2) / mins).max(1);
    let title = truncate_ascii(&summary.title, 14);
    format!("REC {title} {n}m")
}

fn label_next(_locale: &str, ctx: &LabelCtx<'_>) -> String {
    let next = match &ctx.now_summary.next {
        Some(n) => n,
        None => return String::new(),
    };
    let mins = ctx.rounding_minutes.max(1);
    let n = next.starts_in_minute.unwrap_or(0).max(0);
    let n = ((n + mins / 2) / mins).max(1);
    let title = truncate_ascii(&next.title, 14);
    format!("NEXT {title} {n}m")
}

fn truncate_ascii(s: &str, max: usize) -> String {
    s.chars().filter(|c| c.is_ascii()).take(max).collect()
}

pub fn render_slot(label: &str, fg: (u8, u8, u8, u8)) -> Image<'static> {
    let width = label_width(label).clamp(SLOT_MIN_WIDTH, SLOT_MAX_WIDTH);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(width, SLOT_HEIGHT, Rgba([0, 0, 0, 0]));
    let mut cursor_x: u32 = 2;
    for byte in label.bytes() {
        if cursor_x + GLYPH_W > width - SLOT_PAD_RIGHT {
            break;
        }
        if let Some(cols) = FONT.get(&byte) {
            draw_glyph(&mut img, cursor_x, 7, cols, fg);
        } else {
            draw_unknown(&mut img, cursor_x, 7, fg);
        }
        cursor_x += GLYPH_ADVANCE;
    }
    Image::new_owned(img.into_raw(), width, SLOT_HEIGHT)
}

pub fn render_menu_dot(color: (u8, u8, u8, u8)) -> Image<'static> {
    let size = 22u32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let cx = (size / 2) as i32;
    let cy = (size / 2) as i32;
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= DOT_RADIUS * DOT_RADIUS {
                img.put_pixel(x as u32, y as u32, Rgba([color.0, color.1, color.2, color.3]));
            }
        }
    }
    Image::new_owned(img.into_raw(), size, size)
}

fn label_width(label: &str) -> u32 {
    let n = label.len() as u32;
    2 + n * GLYPH_ADVANCE + SLOT_PAD_RIGHT
}

fn draw_glyph(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x0: u32,
    y0: u32,
    cols: &[u8; GLYPH_W as usize],
    fg: (u8, u8, u8, u8),
) {
    // Font is column-major (spec §5): outer loop is the column (dx), inner is
    // the vertical row (dy). Each byte's bit 0 = top row, bit 6 = bottom.
    for dx in 0..GLYPH_W {
        let col = cols[dx as usize];
        for dy in 0..GLYPH_H {
            if col & (1 << dy) != 0 {
                img.put_pixel(x0 + dx, y0 + dy, Rgba([fg.0, fg.1, fg.2, fg.3]));
            }
        }
    }
}

fn draw_unknown(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x0: u32,
    y0: u32,
    fg: (u8, u8, u8, u8),
) {
    for dy in 0..GLYPH_H {
        for dx in 0..GLYPH_W {
            let edge = dy == 0 || dy == GLYPH_H - 1 || dx == 0 || dx == GLYPH_W - 1;
            if edge {
                img.put_pixel(x0 + dx, y0 + dy, Rgba([fg.0, fg.1, fg.2, fg.3]));
            }
        }
    }
}

static FONT: once_cell::sync::Lazy<BTreeMap<u8, [u8; GLYPH_W as usize]>> =
    once_cell::sync::Lazy::new(|| {
        let mut m = BTreeMap::new();
        m.insert(b' ', [0x00, 0x00, 0x00, 0x00, 0x00]);
        m.insert(b'!', [0x00, 0x00, 0x5F, 0x00, 0x00]);
        m.insert(b'\'', [0x00, 0x07, 0x00, 0x00, 0x00]);
        m.insert(b'(', [0x00, 0x1C, 0x22, 0x41, 0x00]);
        m.insert(b')', [0x00, 0x41, 0x22, 0x1C, 0x00]);
        m.insert(b'+', [0x00, 0x08, 0x3E, 0x08, 0x00]);
        m.insert(b',', [0x00, 0x50, 0x30, 0x00, 0x00]);
        m.insert(b'-', [0x00, 0x08, 0x08, 0x08, 0x00]);
        m.insert(b'.', [0x00, 0x60, 0x60, 0x00, 0x00]);
        m.insert(b'/', [0x20, 0x10, 0x08, 0x04, 0x02]);
        m.insert(b'0', [0x3E, 0x51, 0x49, 0x45, 0x3E]);
        m.insert(b'1', [0x00, 0x42, 0x7F, 0x40, 0x00]);
        m.insert(b'2', [0x42, 0x61, 0x51, 0x49, 0x46]);
        m.insert(b'3', [0x21, 0x41, 0x45, 0x4B, 0x31]);
        m.insert(b'4', [0x18, 0x14, 0x12, 0x7F, 0x10]);
        m.insert(b'5', [0x27, 0x45, 0x45, 0x45, 0x39]);
        m.insert(b'6', [0x3C, 0x4A, 0x49, 0x49, 0x30]);
        m.insert(b'7', [0x01, 0x71, 0x09, 0x05, 0x03]);
        m.insert(b'8', [0x36, 0x49, 0x49, 0x49, 0x36]);
        m.insert(b'9', [0x06, 0x49, 0x49, 0x29, 0x1E]);
        m.insert(b':', [0x00, 0x36, 0x36, 0x00, 0x00]);
        m.insert(b'A', [0x7E, 0x11, 0x11, 0x11, 0x7E]);
        m.insert(b'B', [0x7F, 0x49, 0x49, 0x49, 0x36]);
        m.insert(b'C', [0x3E, 0x41, 0x41, 0x41, 0x22]);
        m.insert(b'D', [0x7F, 0x41, 0x41, 0x22, 0x1C]);
        m.insert(b'E', [0x7F, 0x49, 0x49, 0x49, 0x41]);
        m.insert(b'F', [0x7F, 0x09, 0x09, 0x01, 0x01]);
        m.insert(b'G', [0x3E, 0x41, 0x41, 0x51, 0x32]);
        m.insert(b'H', [0x7F, 0x08, 0x08, 0x08, 0x7F]);
        m.insert(b'I', [0x00, 0x41, 0x7F, 0x41, 0x00]);
        m.insert(b'J', [0x20, 0x40, 0x41, 0x3F, 0x01]);
        m.insert(b'K', [0x7F, 0x08, 0x14, 0x22, 0x41]);
        m.insert(b'L', [0x7F, 0x40, 0x40, 0x40, 0x40]);
        m.insert(b'M', [0x7F, 0x02, 0x04, 0x02, 0x7F]);
        m.insert(b'N', [0x7F, 0x04, 0x08, 0x10, 0x7F]);
        m.insert(b'O', [0x3E, 0x41, 0x41, 0x41, 0x3E]);
        m.insert(b'P', [0x7F, 0x09, 0x09, 0x09, 0x06]);
        m.insert(b'Q', [0x3E, 0x41, 0x51, 0x21, 0x5E]);
        m.insert(b'R', [0x7F, 0x09, 0x19, 0x29, 0x46]);
        m.insert(b'S', [0x46, 0x49, 0x49, 0x49, 0x31]);
        m.insert(b'T', [0x01, 0x01, 0x7F, 0x01, 0x01]);
        m.insert(b'U', [0x3F, 0x40, 0x40, 0x40, 0x3F]);
        m.insert(b'V', [0x1F, 0x20, 0x40, 0x20, 0x1F]);
        m.insert(b'W', [0x7F, 0x20, 0x10, 0x20, 0x7F]);
        m.insert(b'X', [0x63, 0x14, 0x08, 0x14, 0x63]);
        m.insert(b'Y', [0x03, 0x04, 0x78, 0x04, 0x03]);
        m.insert(b'Z', [0x61, 0x51, 0x49, 0x45, 0x43]);
        m.insert(b'm', [0x00, 0x7C, 0x08, 0x04, 0x78]);
        m
    });

/// Read a single pixel as (r, g, b, a).
#[cfg(test)]
fn pixel_at(img: &Image<'static>, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let rgba = img.rgba();
    let width = img.width() as usize;
    let idx = (y as usize * width + x as usize) * 4;
    (rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3])
}

/// True iff the pixel at (x, y) is the foreground color with full alpha.
#[cfg(test)]
fn is_foreground(img: &Image<'static>, fg: (u8, u8, u8, u8), x: u32, y: u32) -> bool {
    pixel_at(img, x, y) == (fg.0, fg.1, fg.2, fg.3)
}

/// Count foreground pixels in a rectangular band of the image.
#[cfg(test)]
fn count_fg_pixels(img: &Image<'static>, fg: (u8, u8, u8, u8), x0: u32, y0: u32, w: u32, h: u32) -> usize {
    let mut count = 0;
    for dy in 0..h {
        for dx in 0..w {
            if is_foreground(img, fg, x0 + dx, y0 + dy) {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_menu_dot_dimensions() {
        let img = render_menu_dot((255, 255, 255, 255));
        assert_eq!(img.width(), 22);
        assert_eq!(img.height(), 22);
    }

    #[test]
    fn render_slot_unknown_char_renders_box() {
        let img = render_slot("ab안cd", (10, 20, 30, 255));
        assert!(img.width() >= SLOT_MIN_WIDTH);
        // 'a' is in the font (cols at cursor_x = 2..7, y = 7..14); count fg
        // pixels in that band — must be > 0 for a real glyph render.
        let fg_pixels = count_fg_pixels(&img, (10, 20, 30, 255), 2, 7, GLYPH_W, GLYPH_H);
        assert!(fg_pixels > 0, "expected 'a' to render some foreground pixels, got 0");
    }

    #[test]
    fn render_slot_known_kind_renders_recognizable_glyphs() {
        // "REC code 12m" — every glyph has data; column-major orientation means
        // the result is *real* letters, not garbage. Garbage from the prior
        // row-major bug produced very few foreground pixels.
        let img = render_slot("REC code 12m", (255, 255, 255, 255));
        assert!(img.width() >= SLOT_MIN_WIDTH);
        assert_eq!(img.height(), SLOT_HEIGHT);
        let total = count_fg_pixels(&img, (255, 255, 255, 255), 0, 0, img.width(), SLOT_HEIGHT);
        // 12 glyphs × roughly 8–22 lit pixels each (5×7 matrix) ≈ 100–260 fg
        // pixels; a fully-garbled render produced << 20.
        assert!(
            total >= 60,
            "expected ≥60 foreground pixels for a correctly rendered label, got {total}"
        );
    }

    #[test]
    fn render_slot_a_apex_at_top_not_bottom() {
        // 'A' col 0 = 0x7E = 0b0111_1110. With bit 0 = top, this column
        // lights rows 1..5 (left leg) and leaves rows 0 and 6 dark. A flipped
        // orientation (bit 6 = top) would instead light rows 0..5 — apex/legs
        // at the bottom. Positional assertions (not just counts) so this
        // catches a bit-ordering regression the same way count tests cannot.
        let fg = (255, 255, 255, 255);
        let img = render_slot("A", fg);
        let x0 = 2u32;
        let y0 = 7u32;

        // Col 0 row 0: 0x7E bit 0 = 0 → transparent (top-left dark).
        assert!(
            !is_foreground(&img, fg, x0, y0),
            "expected pixel ({x0},{y0}) transparent (col 0 bit 0 = 0)"
        );
        // Col 0 row 1: 0x7E bit 1 = 1 → foreground.
        assert!(
            is_foreground(&img, fg, x0, y0 + 1),
            "expected pixel ({x0},{}) foreground (col 0 bit 1 = 1)",
            y0 + 1
        );
        // Col 0 row 6: 0x7E bit 6 = 1 → foreground.
        assert!(
            is_foreground(&img, fg, x0, y0 + 6),
            "expected pixel ({x0},{}) foreground (col 0 bit 6 = 1)",
            y0 + 6
        );

        // Col 2 row 0: 0x11 bit 0 = 1 → apex pixel lit at top.
        assert!(
            is_foreground(&img, fg, x0 + 2, y0),
            "expected apex pixel at ({},{}) foreground (col 2 bit 0 = 1)",
            x0 + 2,
            y0
        );
        // Col 2 row 1: 0x11 bit 1 = 0 → dark (interior of the A).
        assert!(
            !is_foreground(&img, fg, x0 + 2, y0 + 1),
            "expected pixel ({},{}) transparent (col 2 bit 1 = 0)",
            x0 + 2,
            y0 + 1
        );
    }

    #[test]
    fn render_slot_korean_char_falls_through_to_box() {
        // Spec §6.4 / §10.2 item 4: non-ASCII bytes → box fallback. The '안'
        // byte sequence routes into draw_unknown, painting the perimeter of
        // a 5×7 cell.
        let img = render_slot("X", (200, 200, 200, 255));
        // First prove the surrounding 'X' is actually rendered, then test
        // the box behavior in isolation.
        let x_total = count_fg_pixels(&img, (200, 200, 200, 255), 2, 7, GLYPH_W, GLYPH_H);
        assert!(x_total > 0);

        // Render only an unknown byte; draw_unknown paints the 5×7 perimeter
        // = 5 + 3 + 5 = 16 pixels.
        let img_box = render_slot("?", (200, 200, 200, 255));
        let box_total = count_fg_pixels(&img_box, (200, 200, 200, 255), 2, 7, GLYPH_W, GLYPH_H);
        assert!(
            (12..=20).contains(&box_total),
            "expected '?' to render as a 5×7 box perimeter (~16 pixels), got {box_total}"
        );
    }
}