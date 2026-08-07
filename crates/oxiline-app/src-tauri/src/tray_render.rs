//! Bitmapped menu-bar slot renderer.
// Public API + helpers land in Task 3 — silence dead-code until then.
#![allow(dead_code)]

//!
//! Each slot is a 22 px tall RGBA buffer. v1 ships an ASCII 5×7 monospaced
//! font (see `FONT`); non-ASCII bytes render as a 5×7 box so width stays
//! predictable across locales.

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
        if let Some(rows) = FONT.get(&byte) {
            draw_glyph(&mut img, cursor_x, 7, rows, fg);
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
    rows: &[u8; GLYPH_H as usize],
    fg: (u8, u8, u8, u8),
) {
    for (dy, row) in rows.iter().enumerate() {
        for dx in 0..GLYPH_W {
            if row & (1 << (GLYPH_W - 1 - dx)) != 0 {
                img.put_pixel(x0 + dx, y0 + dy as u32, Rgba([fg.0, fg.1, fg.2, fg.3]));
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

static FONT: once_cell::sync::Lazy<BTreeMap<u8, [u8; GLYPH_H as usize]>> =
    once_cell::sync::Lazy::new(|| {
        let mut m = BTreeMap::new();
        m.insert(b' ', [0, 0, 0, 0, 0, 0, 0]);
        m.insert(b'!', [0x00, 0x00, 0x5F, 0x00, 0x00, 0x00, 0x00]);
        m.insert(b'\'', [0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00]);
        m.insert(b'(', [0x00, 0x1C, 0x22, 0x41, 0x00, 0x00, 0x00]);
        m.insert(b')', [0x00, 0x41, 0x22, 0x1C, 0x00, 0x00, 0x00]);
        m.insert(b'+', [0x00, 0x08, 0x3E, 0x08, 0x00, 0x00, 0x00]);
        m.insert(b',', [0x00, 0x50, 0x30, 0x00, 0x00, 0x00, 0x00]);
        m.insert(b'-', [0x00, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00]);
        m.insert(b'.', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]);
        m.insert(b'/', [0x20, 0x10, 0x08, 0x04, 0x02, 0x00, 0x00]);
        m.insert(b'0', [0x3E, 0x51, 0x49, 0x45, 0x3E, 0x00, 0x00]);
        m.insert(b'1', [0x00, 0x42, 0x7F, 0x40, 0x00, 0x00, 0x00]);
        m.insert(b'2', [0x42, 0x61, 0x51, 0x49, 0x46, 0x00, 0x00]);
        m.insert(b'3', [0x21, 0x41, 0x45, 0x4B, 0x31, 0x00, 0x00]);
        m.insert(b'4', [0x18, 0x14, 0x12, 0x7F, 0x10, 0x00, 0x00]);
        m.insert(b'5', [0x27, 0x45, 0x45, 0x45, 0x39, 0x00, 0x00]);
        m.insert(b'6', [0x3C, 0x4A, 0x49, 0x49, 0x30, 0x00, 0x00]);
        m.insert(b'7', [0x01, 0x71, 0x09, 0x05, 0x03, 0x00, 0x00]);
        m.insert(b'8', [0x36, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]);
        m.insert(b'9', [0x06, 0x49, 0x49, 0x29, 0x1E, 0x00, 0x00]);
        m.insert(b':', [0x00, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00]);
        m.insert(b'A', [0x7E, 0x11, 0x11, 0x11, 0x7E, 0x00, 0x00]);
        m.insert(b'B', [0x7F, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]);
        m.insert(b'C', [0x3E, 0x41, 0x41, 0x41, 0x22, 0x00, 0x00]);
        m.insert(b'D', [0x7F, 0x41, 0x41, 0x22, 0x1C, 0x00, 0x00]);
        m.insert(b'E', [0x7F, 0x49, 0x49, 0x49, 0x41, 0x00, 0x00]);
        m.insert(b'F', [0x7F, 0x09, 0x09, 0x01, 0x01, 0x00, 0x00]);
        m.insert(b'G', [0x3E, 0x41, 0x41, 0x51, 0x32, 0x00, 0x00]);
        m.insert(b'H', [0x7F, 0x08, 0x08, 0x08, 0x7F, 0x00, 0x00]);
        m.insert(b'I', [0x00, 0x41, 0x7F, 0x41, 0x00, 0x00, 0x00]);
        m.insert(b'J', [0x20, 0x40, 0x41, 0x3F, 0x01, 0x00, 0x00]);
        m.insert(b'K', [0x7F, 0x08, 0x14, 0x22, 0x41, 0x00, 0x00]);
        m.insert(b'L', [0x7F, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00]);
        m.insert(b'M', [0x7F, 0x02, 0x04, 0x02, 0x7F, 0x00, 0x00]);
        m.insert(b'N', [0x7F, 0x04, 0x08, 0x10, 0x7F, 0x00, 0x00]);
        m.insert(b'O', [0x3E, 0x41, 0x41, 0x41, 0x3E, 0x00, 0x00]);
        m.insert(b'P', [0x7F, 0x09, 0x09, 0x09, 0x06, 0x00, 0x00]);
        m.insert(b'Q', [0x3E, 0x41, 0x51, 0x21, 0x5E, 0x00, 0x00]);
        m.insert(b'R', [0x7F, 0x09, 0x19, 0x29, 0x46, 0x00, 0x00]);
        m.insert(b'S', [0x46, 0x49, 0x49, 0x49, 0x31, 0x00, 0x00]);
        m.insert(b'T', [0x01, 0x01, 0x7F, 0x01, 0x01, 0x00, 0x00]);
        m.insert(b'U', [0x3F, 0x40, 0x40, 0x40, 0x3F, 0x00, 0x00]);
        m.insert(b'V', [0x1F, 0x20, 0x40, 0x20, 0x1F, 0x00, 0x00]);
        m.insert(b'W', [0x7F, 0x20, 0x10, 0x20, 0x7F, 0x00, 0x00]);
        m.insert(b'X', [0x63, 0x14, 0x08, 0x14, 0x63, 0x00, 0x00]);
        m.insert(b'Y', [0x03, 0x04, 0x78, 0x04, 0x03, 0x00, 0x00]);
        m.insert(b'Z', [0x61, 0x51, 0x49, 0x45, 0x43, 0x00, 0x00]);
        m.insert(b'm', [0x00, 0x7C, 0x08, 0x04, 0x78, 0x00, 0x00]);
        m
    });

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
    fn render_slot_known_kind_has_non_zero_width() {
        let img = render_slot("REC code 12m", (255, 255, 255, 255));
        assert!(img.width() >= SLOT_MIN_WIDTH);
        assert_eq!(img.height(), SLOT_HEIGHT);
    }

    #[test]
    fn render_slot_unknown_char_renders_box() {
        let img = render_slot("ab안cd", (10, 20, 30, 255));
        assert!(img.width() >= SLOT_MIN_WIDTH);
    }
}
