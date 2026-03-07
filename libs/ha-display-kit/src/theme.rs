use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Rgb565,
    pub card_bg: Rgb565,
    pub accent_bg: Rgb565,
    pub value: Rgb565,
    pub label: Rgb565,
    pub unavail: Rgb565,
    pub header: Rgb565,
    pub footer: Rgb565,
    pub error: Rgb565,
    pub boot_status: Rgb565,
    pub boot_bg: Rgb565,
    pub cool: Rgb565,
    pub heat: Rgb565,
    pub auto: Rgb565,
    pub dry: Rgb565,
    pub fan_only: Rgb565,
    pub off: Rgb565,
    pub heat_cool: Rgb565,
}

impl Theme {
    pub const GREEN: Self = Self {
        bg:          Rgb565::new(1, 2, 1),
        card_bg:     Rgb565::new(3, 6, 3),
        accent_bg:   Rgb565::new(0, 20, 0),
        value:       Rgb565::new(0, 63, 0),
        label:       Rgb565::new(12, 48, 12),
        unavail:     Rgb565::new(16, 32, 8),
        header:      Rgb565::WHITE,
        footer:      Rgb565::new(8, 32, 8),
        error:       Rgb565::new(31, 10, 0),
        boot_status: Rgb565::new(10, 40, 10),
        boot_bg:     Rgb565::new(2, 4, 2),
        cool:        Rgb565::new(0, 40, 31),
        heat:        Rgb565::new(31, 20, 0),
        auto:        Rgb565::new(0, 50, 15),
        dry:         Rgb565::new(15, 30, 31),
        fan_only:    Rgb565::new(10, 48, 10),
        off:         Rgb565::new(8, 16, 8),
        heat_cool:   Rgb565::new(20, 30, 10),
    };
}

impl Default for Theme {
    fn default() -> Self {
        Self::GREEN
    }
}
