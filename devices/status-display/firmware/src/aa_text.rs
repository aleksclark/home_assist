//! Anti-aliased text using glyphs pre-rasterized by `tools/gen_aa_fonts.py`.
//! Each glyph is an 8-bit coverage bitmap that is alpha-blended over whatever
//! is already in the [`Canvas`], so text is smooth on any background.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::canvas::Canvas;

pub struct Glyph {
    pub ch: char,
    /// Horizontal advance in 1/16 px.
    pub adv16: u16,
    /// Ink offset from the pen position on the baseline.
    pub x: i8,
    pub y: i8,
    pub w: u8,
    pub h: u8,
    pub off: u32,
}

pub struct AaFont {
    pub glyphs: &'static [Glyph],
    pub data: &'static [u8],
}

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl AaFont {
    fn glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs
            .binary_search_by(|g| g.ch.cmp(&ch))
            .ok()
            .map(|i| &self.glyphs[i])
    }

    /// Advance width of `text` in whole pixels.
    pub fn width(&self, text: &str) -> i32 {
        let adv16: i32 = text.chars().filter_map(|c| self.glyph(c)).map(|g| g.adv16 as i32).sum();
        (adv16 + 8) >> 4
    }

    /// Draws `text` with its baseline at `y`; returns the pen x after the text.
    pub fn draw(&self, c: &mut Canvas, text: &str, x: i32, y: i32, color: Rgb565, align: Align) -> i32 {
        let x = match align {
            Align::Left => x,
            Align::Center => x - self.width(text) / 2,
            Align::Right => x - self.width(text),
        };
        let mut pen16 = x << 4;
        for ch in text.chars() {
            let Some(g) = self.glyph(ch) else { continue };
            if g.w == 0 || g.h == 0 {
                pen16 += g.adv16 as i32;
                continue;
            }
            let gx = ((pen16 + 8) >> 4) + g.x as i32;
            let gy = y + g.y as i32;
            let bits = &self.data[g.off as usize..g.off as usize + g.w as usize * g.h as usize];
            for (row, line) in bits.chunks_exact(g.w as usize).enumerate() {
                for (col, &a) in line.iter().enumerate() {
                    if a != 0 {
                        c.blend(Point::new(gx + col as i32, gy + row as i32), color, a);
                    }
                }
            }
            pen16 += g.adv16 as i32;
        }
        (pen16 + 8) >> 4
    }
}
