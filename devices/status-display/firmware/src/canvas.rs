use core::convert::Infallible;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

/// Off-screen RGB565 framebuffer. Regions are composed here and then pushed to
/// the panel in one contiguous transfer, so updates never flicker.
pub struct Canvas {
    size: Size,
    pixels: Vec<Rgb565>,
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            pixels: vec![Rgb565::BLACK; (size.width * size.height) as usize],
        }
    }

    /// Reuse the allocation for a region of a different size.
    pub fn resize(&mut self, size: Size) {
        self.size = size;
        self.pixels.resize((size.width * size.height) as usize, Rgb565::BLACK);
    }

    pub fn pixels(&self) -> &[Rgb565] {
        &self.pixels
    }

    #[allow(dead_code)]
    pub fn pixel(&self, p: Point) -> Option<Rgb565> {
        self.index(p).map(|i| self.pixels[i])
    }

    /// Blends `color` over the existing pixel with coverage `alpha` (0..=255).
    pub fn blend(&mut self, p: Point, color: Rgb565, alpha: u8) {
        let Some(i) = self.index(p) else { return };
        let bg = self.pixels[i];
        let a = alpha as u32;
        let mix = |f: u8, b: u8| ((f as u32 * a + b as u32 * (255 - a) + 127) / 255) as u8;
        self.pixels[i] = Rgb565::new(mix(color.r(), bg.r()), mix(color.g(), bg.g()), mix(color.b(), bg.b()));
    }

    fn index(&self, p: Point) -> Option<usize> {
        if p.x < 0 || p.y < 0 || p.x as u32 >= self.size.width || p.y as u32 >= self.size.height {
            return None;
        }
        Some(p.y as usize * self.size.width as usize + p.x as usize)
    }
}

impl OriginDimensions for Canvas {
    fn size(&self) -> Size {
        self.size
    }
}

impl DrawTarget for Canvas {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, c) in pixels {
            if let Some(i) = self.index(p) {
                self.pixels[i] = c;
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let area = area.intersection(&self.bounding_box());
        let Some(br) = area.bottom_right() else {
            return Ok(());
        };
        let w = self.size.width as usize;
        for y in area.top_left.y..=br.y {
            let row = y as usize * w;
            self.pixels[row + area.top_left.x as usize..=row + br.x as usize].fill(color);
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.pixels.fill(color);
        Ok(())
    }
}
