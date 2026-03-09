use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::draw::{draw_err, style_large, style_small, txt, txt_center};
use crate::layout::Region;
use crate::theme::Theme;

pub struct BootScreen {
    pub title: &'static str,
    pub screen_center_x: i32,
}

impl BootScreen {
    pub fn new(title: &'static str, screen_center_x: i32) -> Self {
        Self { title, screen_center_x }
    }

    pub fn draw_status<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, theme: &Theme, status: &str, ip: Option<&str>,
    ) -> anyhow::Result<()> {
        d.clear(theme.boot_bg).map_err(draw_err)?;
        txt_center(d, self.title, Point::new(self.screen_center_x, 60), style_large(theme.header))?;
        txt_center(d, status, Point::new(self.screen_center_x, 100), style_small(theme.boot_status))?;
        if let Some(ip) = ip {
            txt_center(d, ip, Point::new(self.screen_center_x, 140), style_large(theme.value))?;
        }
        Ok(())
    }

    pub fn draw_error<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, theme: &Theme, content_region: &Region, error: &anyhow::Error,
    ) -> anyhow::Result<()> {
        crate::draw::fill_rect(d, content_region, theme.bg)?;
        txt(d, "Error fetching data:", Point::new(16, 80), style_small(theme.error))?;
        let msg: String = format!("{}", error).chars().take(50).collect();
        txt(d, &msg, Point::new(16, 100), style_small(theme.error))?;
        txt(d, "Retrying in 30s...", Point::new(16, 140), style_small(theme.label))
    }
}

pub trait Card {
    type Data: PartialEq;

    fn region(&self) -> &Region;

    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, data: &Self::Data, theme: &Theme,
    ) -> anyhow::Result<()>;
}

pub fn needs_redraw<T: PartialEq>(current: &T, previous: &T) -> bool {
    current != previous
}

pub fn clear_screen<D: DrawTarget<Color = Rgb565>>(d: &mut D, theme: &Theme) -> anyhow::Result<()> {
    d.clear(theme.bg).map_err(draw_err)
}
