use esp_idf_hal::gpio::{InputPin, OutputPin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::spi::config::{Config as DeviceConfig, DriverConfig};
use esp_idf_hal::spi::{SpiDeviceDriver, SpiDriver};
use esp_idf_hal::units::FromValueType;

use anyhow::Result;

pub struct Touch<'d> {
    _spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    irq: PinDriver<'d, esp_idf_hal::gpio::Gpio36, esp_idf_hal::gpio::Input>,
}

impl<'d> Touch<'d> {
    pub fn new(
        spi: impl Peripheral<P = esp_idf_hal::spi::SPI3> + 'd,
        clk: impl Peripheral<P = impl OutputPin> + 'd,
        mosi: impl Peripheral<P = impl OutputPin> + 'd,
        miso: impl Peripheral<P = impl InputPin> + 'd,
        cs: impl Peripheral<P = impl OutputPin> + 'd,
        irq: impl Peripheral<P = esp_idf_hal::gpio::Gpio36> + 'd,
    ) -> Result<Self> {
        let bus = SpiDriver::new(spi, clk, mosi, Some(miso), &DriverConfig::new())?;

        let spi_dev = SpiDeviceDriver::new(
            bus,
            Some(cs),
            &DeviceConfig::new()
                .baudrate(1.MHz().into())
                .data_mode(embedded_hal::spi::MODE_0),
        )?;

        let irq_pin = PinDriver::input(irq)?;

        Ok(Self {
            _spi: spi_dev,
            irq: irq_pin,
        })
    }

    pub fn is_touched(&self) -> bool {
        self.irq.is_low()
    }
}
