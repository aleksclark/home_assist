use std::fmt::Write as FmtWrite;

use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X13};
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Text};

use crate::layout::Region;
use crate::theme::Theme;

pub fn draw_err<E>(_: E) -> anyhow::Error {
    anyhow::anyhow!("draw error")
}

pub fn style_large(color: Rgb565) -> MonoTextStyle<'static, Rgb565> {
    MonoTextStyleBuilder::new().font(&FONT_10X20).text_color(color).build()
}

pub fn style_small(color: Rgb565) -> MonoTextStyle<'static, Rgb565> {
    MonoTextStyleBuilder::new().font(&FONT_6X13).text_color(color).build()
}

pub fn txt<D: DrawTarget<Color = Rgb565>>(
    d: &mut D, text: &str, pos: Point, style: MonoTextStyle<'static, Rgb565>,
) -> anyhow::Result<()> {
    Text::new(text, pos, style).draw(d).map_err(draw_err)?;
    Ok(())
}

pub fn txt_center<D: DrawTarget<Color = Rgb565>>(
    d: &mut D, text: &str, pos: Point, style: MonoTextStyle<'static, Rgb565>,
) -> anyhow::Result<()> {
    Text::with_alignment(text, pos, style, Alignment::Center).draw(d).map_err(draw_err)?;
    Ok(())
}

pub fn fill_rect<D: DrawTarget<Color = Rgb565>>(
    d: &mut D, r: &Region, color: Rgb565,
) -> anyhow::Result<()> {
    Rectangle::new(Point::new(r.x, r.y), Size::new(r.w, r.h))
        .into_styled(PrimitiveStyleBuilder::new().fill_color(color).build())
        .draw(d)
        .map_err(draw_err)
}

pub fn fill_card<D: DrawTarget<Color = Rgb565>>(
    d: &mut D, r: &Region, theme: &Theme,
) -> anyhow::Result<()> {
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(r.x, r.y), Size::new(r.w, r.h)),
        Size::new(6, 6),
    )
    .into_styled(PrimitiveStyleBuilder::new().fill_color(theme.card_bg).build())
    .draw(d)
    .map_err(draw_err)
}

pub fn hvac_color(mode: &str, theme: &Theme) -> Rgb565 {
    match mode {
        "cool" => theme.cool,
        "heat" => theme.heat,
        "auto" => theme.auto,
        "dry" => theme.dry,
        "fan_only" => theme.fan_only,
        "off" => theme.off,
        "heat_cool" => theme.heat_cool,
        _ => theme.unavail,
    }
}

pub fn capitalize(s: &str) -> String {
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

pub fn fmt_temp(val: Option<f32>) -> String {
    match val {
        Some(t) => {
            let mut s = String::new();
            write!(s, "{:.0} F", t).unwrap();
            s
        }
        None => "--.- F".into(),
    }
}

pub fn fmt_humidity(val: Option<f32>) -> String {
    match val {
        Some(h) => {
            let mut s = String::new();
            write!(s, "{:.0}% RH", h).unwrap();
            s
        }
        None => "--% RH".into(),
    }
}
