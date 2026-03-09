use std::fmt::Write as FmtWrite;

use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use ha_display_kit::{
    clear_screen, fill_card, fill_rect, style_large, style_small, txt, txt_center, BootScreen,
    Region, Theme,
};

use crate::slots::{MetricKind, SharedSlots, MAX_SLOTS};

const SCREEN_W: u32 = 320;
const SCREEN_H: u32 = 240;
const SCREEN_CX: i32 = 160;

const HEADER: Region = Region::new(0, 0, SCREEN_W, 28);
const FOOTER: Region = Region::new(0, SCREEN_H as i32 - 28, SCREEN_W, 28);
const CONTENT: Region = Region::new(0, 28, SCREEN_W, SCREEN_H - 56);

const SLOT_REGIONS: [Region; MAX_SLOTS] = [
    Region::new(8,   36,  148, 56),
    Region::new(164, 36,  148, 56),
    Region::new(8,   100, 148, 56),
    Region::new(164, 100, 148, 56),
    Region::new(8,   164, 148, 40),
    Region::new(164, 164, 148, 40),
];

fn status_color(value: &str, theme: &Theme) -> Rgb565 {
    match value {
        "cool" => theme.cool,
        "heat" => theme.heat,
        "auto" => theme.auto,
        "dry" => theme.dry,
        "fan_only" => theme.fan_only,
        "off" | "unavailable" | "unknown" => theme.off,
        "heat_cool" => theme.heat_cool,
        "on" | "home" | "open" | "unlocked" | "active" | "playing" => theme.value,
        "idle" | "standby" | "paused" => theme.label,
        _ => theme.unavail,
    }
}

pub struct Dashboard {
    ip: String,
    theme: Theme,
    boot: BootScreen,
    prev_values: [String; MAX_SLOTS],
    prev_labels: [String; MAX_SLOTS],
    prev_kinds: [String; MAX_SLOTS],
    prev_generation: u32,
    first_draw: bool,
}

impl Dashboard {
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            theme: Theme::GREEN,
            boot: BootScreen::new("Status Display", SCREEN_CX),
            prev_values: Default::default(),
            prev_labels: Default::default(),
            prev_kinds: Default::default(),
            prev_generation: 0,
            first_draw: true,
        }
    }

    pub fn draw_boot_status<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, status: &str, ip: Option<&str>,
    ) -> Result<()> {
        self.boot.draw_status(d, &self.theme, status, ip)
    }

    pub fn draw_error<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, error: &anyhow::Error,
    ) -> Result<()> {
        self.boot.draw_error(d, &self.theme, &CONTENT, error)
    }

    pub fn update<D: DrawTarget<Color = Rgb565>>(
        &mut self, d: &mut D, slots: &SharedSlots,
    ) -> Result<()> {
        let mgr = slots.lock().unwrap();
        let gen = mgr.generation();
        if gen == self.prev_generation && !self.first_draw {
            return Ok(());
        }

        if self.first_draw {
            clear_screen(d, &self.theme)?;
            self.draw_header(d)?;
            self.draw_footer(d)?;
            for i in 0..MAX_SLOTS {
                let slot = mgr.slot(i);
                self.draw_slot(d, i, &slot.label, &slot.value, &slot.kind, &slot.unit)?;
                self.prev_values[i] = slot.value.clone();
                self.prev_labels[i] = slot.label.clone();
                self.prev_kinds[i] = slot.kind.as_str().to_string();
            }
            self.first_draw = false;
            self.prev_generation = gen;
            return Ok(());
        }

        for i in 0..MAX_SLOTS {
            let slot = mgr.slot(i);
            let kind_str = slot.kind.as_str().to_string();
            if slot.value != self.prev_values[i]
                || slot.label != self.prev_labels[i]
                || kind_str != self.prev_kinds[i]
            {
                self.draw_slot(d, i, &slot.label, &slot.value, &slot.kind, &slot.unit)?;
                self.prev_values[i] = slot.value.clone();
                self.prev_labels[i] = slot.label.clone();
                self.prev_kinds[i] = kind_str;
            }
        }

        self.prev_generation = gen;
        Ok(())
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        let t = &self.theme;
        fill_rect(d, &HEADER, t.accent_bg)?;
        txt_center(d, "Home Status", Point::new(SCREEN_CX, 20), style_large(t.header))
    }

    fn draw_footer<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        let t = &self.theme;
        fill_rect(d, &FOOTER, t.accent_bg)?;
        let text = format!("WiFi OK  |  {}", self.ip);
        txt_center(d, &text, Point::new(SCREEN_CX, SCREEN_H as i32 - 10), style_small(t.footer))
    }

    fn draw_slot<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, idx: usize, label: &str, value: &str, kind: &MetricKind, unit: &str,
    ) -> Result<()> {
        let t = &self.theme;
        let r = &SLOT_REGIONS[idx];
        fill_card(d, r, t)?;

        let lx = r.x + 8;
        let ly = r.y + 16;

        if !label.is_empty() {
            txt(d, label, Point::new(lx, ly), style_small(t.label))?;
        }

        if value.is_empty() || value == "unavailable" || value == "unknown" {
            txt(d, "---", Point::new(lx, ly + 24), style_large(t.unavail))?;
            return Ok(());
        }

        match kind {
            MetricKind::Numeric => {
                let display_val = if let Ok(v) = value.parse::<f32>() {
                    if unit.is_empty() {
                        let mut s = String::new();
                        write!(s, "{:.1}", v).unwrap();
                        s
                    } else {
                        let mut s = String::new();
                        write!(s, "{:.0} {}", v, unit).unwrap();
                        s
                    }
                } else {
                    value.to_string()
                };
                txt(d, &display_val, Point::new(lx, ly + 24), style_large(t.value))?;
            }
            MetricKind::Text => {
                let truncated: String = value.chars().take(20).collect();
                txt(d, &truncated, Point::new(lx, ly + 24), style_large(t.value))?;
            }
            MetricKind::Status => {
                let color = status_color(value, t);
                let display = capitalize_first(value);
                txt(d, &display, Point::new(lx, ly + 24), style_large(color))?;
            }
        }
        Ok(())
    }
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => {
            let mut r = String::new();
            for ch in f.to_uppercase() {
                r.push(ch);
            }
            r.extend(c);
            r
        }
    }
}
