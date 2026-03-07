use std::fmt::Write as FmtWrite;

use anyhow::Result;
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X13};
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Text};

use crate::ha::DashboardData;

const SCREEN_W: u32 = 320;

const BG: Rgb565 = Rgb565::new(1, 2, 1);
const CARD_BG: Rgb565 = Rgb565::new(3, 6, 3);
const ACCENT_BG: Rgb565 = Rgb565::new(0, 20, 0);

const COLOR_VALUE: Rgb565 = Rgb565::new(0, 63, 0);
const COLOR_LABEL: Rgb565 = Rgb565::new(12, 48, 12);
const COLOR_UNAVAIL: Rgb565 = Rgb565::new(16, 32, 8);
const COLOR_HEADER: Rgb565 = Rgb565::WHITE;
const COLOR_FOOTER: Rgb565 = Rgb565::new(8, 32, 8);
const COLOR_ERROR: Rgb565 = Rgb565::new(31, 10, 0);

const COLOR_COOL: Rgb565 = Rgb565::new(0, 40, 31);
const COLOR_HEAT: Rgb565 = Rgb565::new(31, 20, 0);
const COLOR_AUTO: Rgb565 = Rgb565::new(0, 50, 15);
const COLOR_DRY: Rgb565 = Rgb565::new(15, 30, 31);
const COLOR_FAN_ONLY: Rgb565 = Rgb565::new(10, 48, 10);
const COLOR_OFF: Rgb565 = Rgb565::new(8, 16, 8);
const COLOR_HEAT_COOL: Rgb565 = Rgb565::new(20, 30, 10);

struct Region {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

const HEADER: Region    = Region { x: 0,   y: 0,   w: SCREEN_W, h: 28 };
const KITCHEN: Region   = Region { x: 8,   y: 36,  w: 148,      h: 70 };
const BEDROOM: Region   = Region { x: 164, y: 36,  w: 148,      h: 70 };
const DELLA: Region     = Region { x: 8,   y: 114, w: 304,      h: 60 };
const ECOBEE: Region    = Region { x: 8,   y: 182, w: 304,      h: 26 };
const FOOTER: Region    = Region { x: 0,   y: 212, w: SCREEN_W, h: 28 };

fn style_large(color: Rgb565) -> MonoTextStyle<'static, Rgb565> {
    MonoTextStyleBuilder::new().font(&FONT_10X20).text_color(color).build()
}

fn style_small(color: Rgb565) -> MonoTextStyle<'static, Rgb565> {
    MonoTextStyleBuilder::new().font(&FONT_6X13).text_color(color).build()
}

fn card_style() -> PrimitiveStyle<Rgb565> {
    PrimitiveStyleBuilder::new().fill_color(CARD_BG).build()
}

fn draw_err<E>(_: E) -> anyhow::Error {
    anyhow::anyhow!("draw error")
}

fn txt<D: DrawTarget<Color = Rgb565>>(
    d: &mut D, text: &str, pos: Point, style: MonoTextStyle<'static, Rgb565>,
) -> Result<()> {
    Text::new(text, pos, style).draw(d).map_err(draw_err)?;
    Ok(())
}

fn txt_center<D: DrawTarget<Color = Rgb565>>(
    d: &mut D, text: &str, pos: Point, style: MonoTextStyle<'static, Rgb565>,
) -> Result<()> {
    Text::with_alignment(text, pos, style, Alignment::Center).draw(d).map_err(draw_err)?;
    Ok(())
}

fn fill_rect<D: DrawTarget<Color = Rgb565>>(d: &mut D, r: &Region, color: Rgb565) -> Result<()> {
    Rectangle::new(Point::new(r.x, r.y), Size::new(r.w, r.h))
        .into_styled(PrimitiveStyleBuilder::new().fill_color(color).build())
        .draw(d)
        .map_err(draw_err)
}

fn fill_card<D: DrawTarget<Color = Rgb565>>(d: &mut D, r: &Region) -> Result<()> {
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(r.x, r.y), Size::new(r.w, r.h)),
        Size::new(6, 6),
    )
    .into_styled(card_style())
    .draw(d)
    .map_err(draw_err)
}

fn hvac_color(mode: &str) -> Rgb565 {
    match mode {
        "cool" => COLOR_COOL,
        "heat" => COLOR_HEAT,
        "auto" => COLOR_AUTO,
        "dry" => COLOR_DRY,
        "fan_only" => COLOR_FAN_ONLY,
        "off" => COLOR_OFF,
        "heat_cool" => COLOR_HEAT_COOL,
        _ => COLOR_UNAVAIL,
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => {
            let mut r = String::new();
            for ch in f.to_uppercase() { r.push(ch); }
            r.extend(c);
            r
        }
    }
}

fn fmt_temp(val: Option<f32>) -> String {
    match val {
        Some(t) => { let mut s = String::new(); write!(s, "{:.0} F", t).unwrap(); s }
        None => "--.- F".into(),
    }
}

fn fmt_humidity(val: Option<f32>) -> String {
    match val {
        Some(h) => { let mut s = String::new(); write!(s, "{:.0}% RH", h).unwrap(); s }
        None => "--% RH".into(),
    }
}

pub struct Dashboard {
    ip: String,
    prev: DashboardData,
    first_draw: bool,
}

impl Dashboard {
    pub fn new(ip: String) -> Self {
        Self { ip, prev: DashboardData::default(), first_draw: true }
    }

    pub fn draw_boot_status<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, status: &str, ip: Option<&str>,
    ) -> Result<()> {
        d.clear(Rgb565::new(2, 4, 2)).map_err(draw_err)?;
        txt_center(d, "Status Display", Point::new(160, 60), style_large(COLOR_HEADER))?;
        txt_center(d, status, Point::new(160, 100), style_small(Rgb565::new(10, 40, 10)))?;
        if let Some(ip) = ip {
            txt_center(d, ip, Point::new(160, 140), style_large(COLOR_VALUE))?;
        }
        Ok(())
    }

    pub fn draw_error<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, error: &anyhow::Error,
    ) -> Result<()> {
        let content = Region { x: 0, y: 28, w: SCREEN_W, h: 184 };
        fill_rect(d, &content, BG)?;
        txt(d, "Error fetching data:", Point::new(16, 80), style_small(COLOR_ERROR))?;
        let msg: String = format!("{}", error).chars().take(50).collect();
        txt(d, &msg, Point::new(16, 100), style_small(COLOR_ERROR))?;
        txt(d, "Retrying in 30s...", Point::new(16, 140), style_small(COLOR_LABEL))
    }

    pub fn update<D: DrawTarget<Color = Rgb565>>(
        &mut self, d: &mut D, data: &DashboardData,
    ) -> Result<()> {
        if self.first_draw {
            d.clear(BG).map_err(draw_err)?;
            self.draw_header(d)?;
            self.draw_footer(d)?;
            self.draw_kitchen(d, data)?;
            self.draw_bedroom(d, data)?;
            self.draw_della(d, data)?;
            self.draw_ecobee(d, data)?;
            self.prev = data.clone();
            self.first_draw = false;
            return Ok(());
        }

        if data.kitchen_temp != self.prev.kitchen_temp
            || data.kitchen_humidity != self.prev.kitchen_humidity
        {
            self.draw_kitchen(d, data)?;
        }

        if data.bedroom_temp != self.prev.bedroom_temp
            || data.bedroom_humidity != self.prev.bedroom_humidity
        {
            self.draw_bedroom(d, data)?;
        }

        if data.della_mode != self.prev.della_mode
            || data.della_target_temp != self.prev.della_target_temp
            || data.della_current_temp != self.prev.della_current_temp
            || data.della_fan != self.prev.della_fan
        {
            self.draw_della(d, data)?;
        }

        if data.ecobee_mode != self.prev.ecobee_mode
            || data.ecobee_temp != self.prev.ecobee_temp
            || data.ecobee_humidity != self.prev.ecobee_humidity
        {
            self.draw_ecobee(d, data)?;
        }

        self.prev = data.clone();
        Ok(())
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        fill_rect(d, &HEADER, ACCENT_BG)?;
        txt_center(d, "Home Status", Point::new(160, 20), style_large(COLOR_HEADER))
    }

    fn draw_footer<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        fill_rect(d, &FOOTER, ACCENT_BG)?;
        let text = format!("WiFi OK  |  {}", self.ip);
        txt_center(d, &text, Point::new(160, 230), style_small(COLOR_FOOTER))
    }

    fn draw_kitchen<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        fill_card(d, &KITCHEN)?;
        txt(d, "Kitchen", Point::new(16, 54), style_small(COLOR_LABEL))?;
        let color = if data.kitchen_temp.is_some() { COLOR_VALUE } else { COLOR_UNAVAIL };
        txt(d, &fmt_temp(data.kitchen_temp), Point::new(16, 82), style_large(color))?;
        txt(d, &fmt_humidity(data.kitchen_humidity), Point::new(16, 98), style_small(COLOR_LABEL))
    }

    fn draw_bedroom<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        fill_card(d, &BEDROOM)?;
        txt(d, "Bedroom", Point::new(172, 54), style_small(COLOR_LABEL))?;
        let color = if data.bedroom_temp.is_some() { COLOR_VALUE } else { COLOR_UNAVAIL };
        txt(d, &fmt_temp(data.bedroom_temp), Point::new(172, 82), style_large(color))?;
        txt(d, &fmt_humidity(data.bedroom_humidity), Point::new(172, 98), style_small(COLOR_LABEL))
    }

    fn draw_della<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        fill_card(d, &DELLA)?;
        txt(d, "Della Mini Split", Point::new(16, 132), style_small(COLOR_LABEL))?;

        let mode_str = data.della_mode.as_deref().unwrap_or("---");
        let color = hvac_color(mode_str);
        let main_text = match data.della_target_temp {
            Some(target) => { let mut s = String::new(); write!(s, "{}  {:.0} F", capitalize(mode_str), target).unwrap(); s }
            None => format!("{}  -- F", capitalize(mode_str)),
        };
        txt(d, &main_text, Point::new(16, 160), style_large(color))?;

        let fan_text = match &data.della_fan {
            Some(f) => format!("Fan: {}", f),
            None => "Fan: ---".into(),
        };
        txt(d, &fan_text, Point::new(180, 160), style_small(COLOR_LABEL))
    }

    fn draw_ecobee<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        fill_card(d, &ECOBEE)?;

        let text = match (&data.ecobee_mode, data.ecobee_temp) {
            (Some(mode), Some(temp)) => { let mut s = String::new(); write!(s, "Ecobee: {} {:.0}F", capitalize(mode), temp).unwrap(); s }
            (Some(mode), None) => format!("Ecobee: {}", capitalize(mode)),
            _ => "Ecobee: unavailable".into(),
        };
        let color = hvac_color(data.ecobee_mode.as_deref().unwrap_or(""));
        txt(d, &text, Point::new(16, 200), style_small(color))?;

        if let Some(hum) = data.ecobee_humidity {
            let mut s = String::new();
            write!(s, "{:.0}% RH", hum).unwrap();
            txt(d, &s, Point::new(230, 200), style_small(COLOR_LABEL))?;
        }
        Ok(())
    }
}
