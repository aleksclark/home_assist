//! Flat HVAC dashboard for the 320x240 ILI9341 panel.
//!
//! Layout (from the Stitch design "HVAC 320x240 Wall Dashboard"):
//! a 2x2 grid of room tiles (154x92) above a 312x40 info strip, 4px gutters.
//! Every region is composed in an off-screen [`Canvas`] and pushed to the
//! panel in one transfer, and only when its content changed.
//!
//! This module has no ESP-IDF dependencies so the host-side preview tool can
//! render it pixel-for-pixel.

use std::fmt::Debug;

use anyhow::{anyhow, Result};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, RoundedRectangle};

use crate::aa_fonts;
use crate::aa_text::{AaFont, Align};
use crate::canvas::Canvas;
use crate::state::{Action, BbStatus, DisplayState, SharedState, Thermostat, NUM_SENSORS, NUM_THERMOSTATS};

pub const SCREEN_W: u32 = 320;
pub const SCREEN_H: u32 = 240;

const MARGIN: i32 = 4;
const GAP: i32 = 4;
const TILE_W: u32 = 154;
const TILE_H: u32 = 92;
const STRIP_H: u32 = 40;
const RADIUS: u32 = 4;
const PAD_LEFT: i32 = 10;
const PAD_RIGHT: i32 = 7;

const fn hex(rgb: u32) -> Rgb565 {
    Rgb565::new(
        ((rgb >> 16 & 0xff) >> 3) as u8,
        ((rgb >> 8 & 0xff) >> 2) as u8,
        ((rgb & 0xff) >> 3) as u8,
    )
}

pub const BG: Rgb565 = hex(0x0c0d0e);
const TILE_BG: Rgb565 = hex(0x15181b);
const TILE_BORDER: Rgb565 = hex(0x22272c);
const STRIP_BG: Rgb565 = hex(0x111417);
const STRIP_BORDER: Rgb565 = hex(0x1e2429);
const TEXT: Rgb565 = hex(0xf0f2f5);
const TEXT_SOFT: Rgb565 = hex(0xcbd5e1);
const LABEL: Rgb565 = hex(0x8c96a3);
const MUTED: Rgb565 = hex(0x717c8a);
const FAINT: Rgb565 = hex(0x64748b);
const FAN_TAG_BG: Rgb565 = hex(0x1e2c33);
const FAN_TAG_FG: Rgb565 = hex(0x38bdf8);

const HEAT: Rgb565 = hex(0xf97316);
const COOL: Rgb565 = hex(0x38bdf8);
const FAN: Rgb565 = hex(0x14b8a6);
const DRY: Rgb565 = hex(0xeab308);
const DEFROST: Rgb565 = hex(0xc4b5fd);
const IDLE_BAR: Rgb565 = hex(0x475569);
const OFF: Rgb565 = hex(0x3a414a);

const OK_GREEN: Rgb565 = hex(0x22c55e);
const LINK_OK: Rgb565 = hex(0x10b981);
const LINK_DOWN: Rgb565 = hex(0xef4444);
const AWAY: Rgb565 = hex(0xeab308);
const PLAYING: Rgb565 = hex(0xa855f7);

static FONT_TEMP: &AaFont = &aa_fonts::TEMP;
static FONT_UNIT: &AaFont = &aa_fonts::UNIT;
static FONT_BOLD: &AaFont = &aa_fonts::LABEL;
static FONT_REG: &AaFont = &aa_fonts::SMALL;
static FONT_VALUE: &AaFont = &aa_fonts::SMALL_BOLD;
static FONT_BOOT: &AaFont = &aa_fonts::BOOT;

/// (accent bar, label) colors for an action.
fn action_colors(action: Action) -> (Rgb565, Rgb565) {
    match action {
        Action::Heating => (HEAT, HEAT),
        Action::Cooling => (COOL, COOL),
        Action::Fan => (FAN, FAN),
        Action::Drying => (DRY, DRY),
        Action::Defrosting => (DEFROST, DEFROST),
        Action::Idle | Action::Auto => (IDLE_BAR, FAINT),
        Action::Off | Action::Unknown => (OFF, FAINT),
    }
}

fn parse_temp(raw: &str) -> Option<f32> {
    raw.trim().parse::<f32>().ok().filter(|v| v.is_finite())
}

fn fmt_deg(raw: &str) -> Option<String> {
    parse_temp(raw).map(|v| format!("{:.0}", v))
}

#[derive(Clone, Debug, PartialEq)]
pub struct TileView {
    pub name: &'static str,
    pub temp: Option<String>,
    pub action: Action,
    pub fan: bool,
    pub setpoint: String,
}

impl TileView {
    pub fn from_thermostat(t: &Thermostat) -> Self {
        let setpoint = match (fmt_deg(&t.target_low), fmt_deg(&t.target_high)) {
            (Some(lo), Some(hi)) => format!("{}-{}°", lo, hi),
            _ => fmt_deg(&t.target_temp).map(|v| format!("{}°", v)).unwrap_or_default(),
        };
        Self {
            name: t.name,
            temp: fmt_deg(&t.current_temp),
            action: t.action(),
            fan: t.fan_on(),
            setpoint,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FooterView {
    pub band: String,
    pub sensors: [(&'static str, String); NUM_SENSORS],
    pub bb: BbStatus,
    pub ha_connected: bool,
    pub ip: String,
}

impl FooterView {
    pub fn from_state(st: &DisplayState, ip: &str, hour: u8, minute: u8) -> Self {
        let (heat, cool) = st.schedule.current(hour, minute);
        let band = match (fmt_deg(heat), fmt_deg(cool)) {
            (Some(h), Some(c)) => format!("{}-{}°", h, c),
            _ => "--".into(),
        };
        let sensors = core::array::from_fn(|i| {
            let s = &st.sensors[i];
            (s.name, fmt_deg(&s.value).map(|v| format!("{}°", v)).unwrap_or_else(|| "--".into()))
        });
        Self {
            band,
            sensors,
            bb: st.bb_status,
            ha_connected: st.ha_clients > 0,
            ip: ip.to_string(),
        }
    }
}

pub fn tile_rect(idx: usize) -> Rectangle {
    let col = (idx % 2) as i32;
    let row = (idx / 2) as i32;
    Rectangle::new(
        Point::new(MARGIN + col * (TILE_W as i32 + GAP), MARGIN + row * (TILE_H as i32 + GAP)),
        Size::new(TILE_W, TILE_H),
    )
}

pub fn strip_rect() -> Rectangle {
    let y = MARGIN + 2 * (TILE_H as i32 + GAP);
    Rectangle::new(Point::new(MARGIN, y), Size::new(SCREEN_W - 2 * MARGIN as u32, STRIP_H))
}

fn text(c: &mut Canvas, font: &AaFont, s: &str, x: i32, baseline: i32, color: Rgb565) -> i32 {
    font.draw(c, s, x, baseline, color, Align::Left)
}

fn text_right(c: &mut Canvas, font: &AaFont, s: &str, right: i32, baseline: i32, color: Rgb565) {
    font.draw(c, s, right, baseline, color, Align::Right);
}

fn width(font: &AaFont, s: &str) -> i32 {
    font.width(s)
}

const SS: i32 = 4;

/// Fraction (0..=255) of pixel (px, py) inside the rounded rectangle spanning
/// [inset, w - inset) x [inset, h - inset) with corner radius `r`.
fn rounded_coverage(px: i32, py: i32, w: i32, h: i32, inset: i32, r: f32) -> u8 {
    let (x0, y0) = (inset as f32 + r, inset as f32 + r);
    let (x1, y1) = ((w - inset) as f32 - r, (h - inset) as f32 - r);
    let mut hits = 0;
    for sy in 0..SS {
        for sx in 0..SS {
            let x = px as f32 + (sx as f32 + 0.5) / SS as f32;
            let y = py as f32 + (sy as f32 + 0.5) / SS as f32;
            let dx = x - x.clamp(x0, x1);
            let dy = y - y.clamp(y0, y1);
            if x >= inset as f32 && y >= inset as f32 && x <= (w - inset) as f32
                && y <= (h - inset) as f32 && dx * dx + dy * dy <= r * r
            {
                hits += 1;
            }
        }
    }
    (hits * 255 / (SS * SS)) as u8
}

/// Flat panel with a 1px border and anti-aliased rounded corners.
fn panel(c: &mut Canvas, fill: Rgb565, border: Rgb565) {
    let size = c.size();
    let (w, h) = (size.width as i32, size.height as i32);
    c.clear(BG).ok();
    c.fill_solid(&Rectangle::new(Point::zero(), size), border).ok();
    c.fill_solid(&Rectangle::new(Point::new(1, 1), Size::new(size.width - 2, size.height - 2)), fill).ok();
    let r = RADIUS as i32;
    for (cx, cy) in [(0, 0), (w - r, 0), (0, h - r), (w - r, h - r)] {
        for py in cy..cy + r {
            for px in cx..cx + r {
                let p = Point::new(px, py);
                c.fill_solid(&Rectangle::new(p, Size::new(1, 1)), BG).ok();
                c.blend(p, border, rounded_coverage(px, py, w, h, 0, RADIUS as f32));
                c.blend(p, fill, rounded_coverage(px, py, w, h, 1, RADIUS as f32 - 1.0));
            }
        }
    }
}

/// Accent bar along the left edge, clipped to the panel's rounded corners.
fn accent_bar(c: &mut Canvas, color: Rgb565, width: i32) {
    let size = c.size();
    let (w, h) = (size.width as i32, size.height as i32);
    for py in 0..h {
        for px in 0..width {
            c.blend(Point::new(px, py), color, rounded_coverage(px, py, w, h, 0, RADIUS as f32));
        }
    }
}

fn dot(c: &mut Canvas, x: i32, center_y: i32, color: Rgb565) {
    const R: f32 = 2.6;
    let (cx, cy) = (x as f32 + 2.5, center_y as f32 + 0.5);
    for py in center_y - 3..=center_y + 3 {
        for px in x - 1..=x + 5 {
            let mut hits = 0;
            for sy in 0..SS {
                for sx in 0..SS {
                    let dx = px as f32 + (sx as f32 + 0.5) / SS as f32 - cx;
                    let dy = py as f32 + (sy as f32 + 0.5) / SS as f32 - cy;
                    if dx * dx + dy * dy <= R * R {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                c.blend(Point::new(px, py), color, (hits * 255 / (SS * SS)) as u8);
            }
        }
    }
}

/// Draws one room tile into `c`, which must be sized to the tile.
pub fn draw_tile(c: &mut Canvas, v: &TileView) {
    c.resize(Size::new(TILE_W, TILE_H));
    panel(c, TILE_BG, TILE_BORDER);
    let w = TILE_W as i32;
    let h = TILE_H as i32;
    let (bar, label) = action_colors(v.action);

    accent_bar(c, bar, 3);

    text(c, FONT_BOLD, &v.name.to_uppercase(), PAD_LEFT, 15, LABEL);

    if v.fan {
        let tw = width(FONT_BOLD, "FAN");
        let tag = Rectangle::new(Point::new(w - PAD_RIGHT - tw - 6, 5), Size::new(tw as u32 + 6, 13));
        RoundedRectangle::with_equal_corners(tag, Size::new(2, 2))
            .into_styled(PrimitiveStyle::with_fill(FAN_TAG_BG))
            .draw(c)
            .ok();
        text(c, FONT_BOLD, "FAN", tag.top_left.x + 3, 15, FAN_TAG_FG);
    }

    match &v.temp {
        Some(temp) => {
            let end = text(c, FONT_TEMP, temp, PAD_LEFT, 61, TEXT);
            text(c, FONT_UNIT, "°F", end + 3, 61, LABEL);
        }
        None => {
            text(c, FONT_TEMP, "--", PAD_LEFT, 61, OFF);
        }
    }

    let footer_y = h - 9;
    dot(c, PAD_LEFT, footer_y - 4, label);
    text(c, FONT_BOLD, v.action.label(), PAD_LEFT + 8, footer_y, label);
    text_right(c, FONT_REG, &v.setpoint, w - PAD_RIGHT, footer_y, MUTED);
}

/// Draws the bottom info strip into `c`, which must be sized to the strip.
pub fn draw_strip(c: &mut Canvas, v: &FooterView) {
    let size = strip_rect().size;
    c.resize(size);
    panel(c, STRIP_BG, STRIP_BORDER);
    let w = size.width as i32;
    let row1 = 16;
    let row2 = 32;

    let x = text(c, FONT_REG, "Band ", 7, row1, LABEL);
    text(c, FONT_VALUE, &v.band, x, row1, TEXT_SOFT);

    let gap = "   ";
    let total: i32 = v
        .sensors
        .iter()
        .enumerate()
        .map(|(i, (name, val))| {
            width(FONT_REG, &format!("{} ", name))
                + width(FONT_VALUE, val)
                + if i > 0 { width(FONT_REG, gap) } else { 0 }
        })
        .sum();
    let mut x = w - PAD_RIGHT - total;
    for (i, (name, val)) in v.sensors.iter().enumerate() {
        if i > 0 {
            x = text(c, FONT_REG, gap, x, row1, LABEL);
        }
        x = text(c, FONT_REG, &format!("{} ", name), x, row1, LABEL);
        x = text(c, FONT_VALUE, val, x, row1, TEXT);
    }

    let bb_color = match v.bb {
        BbStatus::Working => OK_GREEN,
        BbStatus::Away => AWAY,
        BbStatus::Playing => PLAYING,
        BbStatus::Unknown => FAINT,
    };
    dot(c, 7, row2 - 4, bb_color);
    text(c, FONT_REG, &format!("BB: {}", v.bb.label()), 15, row2, TEXT_SOFT);

    let (link_color, link_text) = if v.ha_connected {
        (LINK_OK, v.ip.clone())
    } else {
        (LINK_DOWN, "HA offline".to_string())
    };
    let tw = width(FONT_REG, &link_text);
    dot(c, w - PAD_RIGHT - tw - 8, row2 - 4, link_color);
    text_right(c, FONT_REG, &link_text, w - PAD_RIGHT, row2, FAINT);
}

fn blit<D>(d: &mut D, area: &Rectangle, c: &Canvas) -> Result<()>
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: Debug,
{
    d.fill_contiguous(area, c.pixels().iter().copied())
        .map_err(|e| anyhow!("blit: {:?}", e))
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Fresh,
    Boot,
    Dashboard,
}

pub struct Dashboard {
    ip: String,
    canvas: Canvas,
    tiles: [Option<TileView>; NUM_THERMOSTATS],
    strip: Option<FooterView>,
    mode: Mode,
    /// (state generation, hour, minute) of the last completed update.
    last_key: Option<(u32, u8, u8)>,
}

impl Dashboard {
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            canvas: Canvas::new(Size::new(TILE_W, TILE_H)),
            tiles: Default::default(),
            strip: None,
            mode: Mode::Fresh,
            last_key: None,
        }
    }

    pub fn set_ip(&mut self, ip: String) {
        self.ip = ip;
        self.last_key = None;
    }

    /// Full-screen boot/progress message.
    pub fn draw_boot_status<D>(&mut self, d: &mut D, status: &str, ip: Option<&str>) -> Result<()>
    where
        D: DrawTarget<Color = Rgb565>,
        D::Error: Debug,
    {
        let size = Size::new(SCREEN_W, 64);
        let area = Rectangle::new(Point::new(0, (SCREEN_H as i32 - 64) / 2), size);
        self.canvas.resize(size);
        let c = &mut self.canvas;
        c.clear(BG).ok();
        let cx = SCREEN_W as i32 / 2;
        let center = |c: &mut Canvas, f: &AaFont, s: &str, y: i32, color: Rgb565| {
            f.draw(c, s, cx, y, color, Align::Center);
        };
        center(c, FONT_BOLD, "STATUS DISPLAY", 14, LABEL);
        center(c, FONT_BOOT, status, 38, TEXT);
        if let Some(ip) = ip {
            center(c, FONT_REG, ip, 56, FAINT);
        }
        if self.mode != Mode::Boot {
            d.clear(BG).map_err(|e| anyhow!("clear: {:?}", e))?;
            self.mode = Mode::Boot;
        }
        blit(d, &area, &self.canvas)
    }

    /// Redraws whichever regions changed since the last call.
    pub fn update<D>(&mut self, d: &mut D, state: &SharedState, hour: u8, minute: u8) -> Result<()>
    where
        D: DrawTarget<Color = Rgb565>,
        D::Error: Debug,
    {
        let key = {
            let st = state.lock().map_err(|_| anyhow!("state lock poisoned"))?;
            (st.generation(), hour, minute)
        };
        if self.mode == Mode::Dashboard && self.last_key == Some(key) {
            return Ok(());
        }
        let (tiles, strip) = {
            let st = state.lock().map_err(|_| anyhow!("state lock poisoned"))?;
            let tiles: [TileView; NUM_THERMOSTATS] =
                core::array::from_fn(|i| TileView::from_thermostat(&st.thermostats[i]));
            (tiles, FooterView::from_state(&st, &self.ip, hour, minute))
        };
        self.render(d, tiles, strip)?;
        self.last_key = Some(key);
        Ok(())
    }

    pub fn render<D>(
        &mut self,
        d: &mut D,
        tiles: [TileView; NUM_THERMOSTATS],
        strip: FooterView,
    ) -> Result<()>
    where
        D: DrawTarget<Color = Rgb565>,
        D::Error: Debug,
    {
        if self.mode != Mode::Dashboard {
            d.clear(BG).map_err(|e| anyhow!("clear: {:?}", e))?;
            self.tiles = Default::default();
            self.strip = None;
            self.mode = Mode::Dashboard;
        }
        for (i, view) in tiles.into_iter().enumerate() {
            if self.tiles[i].as_ref() != Some(&view) {
                draw_tile(&mut self.canvas, &view);
                blit(d, &tile_rect(i), &self.canvas)?;
                self.tiles[i] = Some(view);
            }
        }
        if self.strip.as_ref() != Some(&strip) {
            draw_strip(&mut self.canvas, &strip);
            blit(d, &strip_rect(), &self.canvas)?;
            self.strip = Some(strip);
        }
        Ok(())
    }
}
