use std::fmt::Write as FmtWrite;

use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use ha_display_kit::{
    capitalize, fill_card, fill_rect, fmt_humidity, fmt_temp, hvac_color, needs_redraw,
    style_large, style_small, txt, txt_center, BootScreen, Region, Theme,
};

use crate::ha::DashboardData;

const SCREEN_W: u32 = 320;
const SCREEN_CENTER_X: i32 = 160;

const HEADER: Region  = Region::new(0,   0,   SCREEN_W, 28);
const KITCHEN: Region = Region::new(8,   36,  148,      70);
const BEDROOM: Region = Region::new(164, 36,  148,      70);
const DELLA: Region   = Region::new(8,   114, 304,      60);
const ECOBEE: Region  = Region::new(8,   182, 304,      26);
const FOOTER: Region  = Region::new(0,   212, SCREEN_W, 28);
const CONTENT: Region = Region::new(0,   28,  SCREEN_W, 184);

pub struct Dashboard {
    ip: String,
    theme: Theme,
    boot: BootScreen,
    prev: DashboardData,
    first_draw: bool,
}

impl Dashboard {
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            theme: Theme::GREEN,
            boot: BootScreen::new("Status Display", SCREEN_CENTER_X),
            prev: DashboardData::default(),
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
        &mut self, d: &mut D, data: &DashboardData,
    ) -> Result<()> {
        let t = &self.theme;

        if self.first_draw {
            ha_display_kit::clear_screen(d, t)?;
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

        if needs_redraw(
            &(data.kitchen_temp, data.kitchen_humidity),
            &(self.prev.kitchen_temp, self.prev.kitchen_humidity),
        ) {
            self.draw_kitchen(d, data)?;
        }

        if needs_redraw(
            &(data.bedroom_temp, data.bedroom_humidity),
            &(self.prev.bedroom_temp, self.prev.bedroom_humidity),
        ) {
            self.draw_bedroom(d, data)?;
        }

        if needs_redraw(
            &(&data.della_mode, data.della_target_temp, data.della_current_temp, &data.della_fan),
            &(&self.prev.della_mode, self.prev.della_target_temp, self.prev.della_current_temp, &self.prev.della_fan),
        ) {
            self.draw_della(d, data)?;
        }

        if needs_redraw(
            &(&data.ecobee_mode, data.ecobee_temp, data.ecobee_humidity),
            &(&self.prev.ecobee_mode, self.prev.ecobee_temp, self.prev.ecobee_humidity),
        ) {
            self.draw_ecobee(d, data)?;
        }

        self.prev = data.clone();
        Ok(())
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        fill_rect(d, &HEADER, self.theme.accent_bg)?;
        txt_center(d, "Home Status", Point::new(SCREEN_CENTER_X, 20), style_large(self.theme.header))
    }

    fn draw_footer<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        fill_rect(d, &FOOTER, self.theme.accent_bg)?;
        let text = format!("WiFi OK  |  {}", self.ip);
        txt_center(d, &text, Point::new(SCREEN_CENTER_X, 230), style_small(self.theme.footer))
    }

    fn draw_kitchen<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        let t = &self.theme;
        fill_card(d, &KITCHEN, t)?;
        txt(d, "Kitchen", Point::new(16, 54), style_small(t.label))?;
        let color = if data.kitchen_temp.is_some() { t.value } else { t.unavail };
        txt(d, &fmt_temp(data.kitchen_temp), Point::new(16, 82), style_large(color))?;
        txt(d, &fmt_humidity(data.kitchen_humidity), Point::new(16, 98), style_small(t.label))
    }

    fn draw_bedroom<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        let t = &self.theme;
        fill_card(d, &BEDROOM, t)?;
        txt(d, "Bedroom", Point::new(172, 54), style_small(t.label))?;
        let color = if data.bedroom_temp.is_some() { t.value } else { t.unavail };
        txt(d, &fmt_temp(data.bedroom_temp), Point::new(172, 82), style_large(color))?;
        txt(d, &fmt_humidity(data.bedroom_humidity), Point::new(172, 98), style_small(t.label))
    }

    fn draw_della<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        let t = &self.theme;
        fill_card(d, &DELLA, t)?;
        txt(d, "Della Mini Split", Point::new(16, 132), style_small(t.label))?;

        let mode_str = data.della_mode.as_deref().unwrap_or("---");
        let color = hvac_color(mode_str, t);
        let main_text = match data.della_target_temp {
            Some(target) => {
                let mut s = String::new();
                write!(s, "{}  {:.0} F", capitalize(mode_str), target).unwrap();
                s
            }
            None => format!("{}  -- F", capitalize(mode_str)),
        };
        txt(d, &main_text, Point::new(16, 160), style_large(color))?;

        let fan_text = match &data.della_fan {
            Some(f) => format!("Fan: {}", f),
            None => "Fan: ---".into(),
        };
        txt(d, &fan_text, Point::new(180, 160), style_small(t.label))
    }

    fn draw_ecobee<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, data: &DashboardData) -> Result<()> {
        let t = &self.theme;
        fill_card(d, &ECOBEE, t)?;

        let text = match (&data.ecobee_mode, data.ecobee_temp) {
            (Some(mode), Some(temp)) => {
                let mut s = String::new();
                write!(s, "Ecobee: {} {:.0}F", capitalize(mode), temp).unwrap();
                s
            }
            (Some(mode), None) => format!("Ecobee: {}", capitalize(mode)),
            _ => "Ecobee: unavailable".into(),
        };
        let color = hvac_color(data.ecobee_mode.as_deref().unwrap_or(""), t);
        txt(d, &text, Point::new(16, 200), style_small(color))?;

        if let Some(hum) = data.ecobee_humidity {
            let mut s = String::new();
            write!(s, "{:.0}% RH", hum).unwrap();
            txt(d, &s, Point::new(230, 200), style_small(t.label))?;
        }
        Ok(())
    }
}
