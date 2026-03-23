use std::time::{Duration, Instant};

use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::units::FromValueType;

use anyhow::Result;

const DIM_TIMEOUT: Duration = Duration::from_secs(180);
const OFF_TIMEOUT: Duration = Duration::from_secs(300);

const DUTY_FULL: u8 = 100;
const DUTY_DIM: u8 = 30;
const DUTY_OFF: u8 = 0;


#[derive(Clone, Copy, PartialEq)]
enum BrightLevel {
    Full,
    Dim,
    Off,
}

pub struct Backlight<'d> {
    driver: LedcDriver<'d>,
    max_duty: u32,
    level: BrightLevel,
    last_activity: Instant,
    was_touched: bool,

}

impl<'d> Backlight<'d> {
    pub fn new(
        timer: impl Peripheral<P = esp_idf_hal::ledc::TIMER0> + 'd,
        channel: impl Peripheral<P = esp_idf_hal::ledc::CHANNEL0> + 'd,
        pin: impl Peripheral<P = impl esp_idf_hal::gpio::OutputPin> + 'd,
    ) -> Result<Self> {
        let timer_driver = LedcTimerDriver::new(
            timer,
            &TimerConfig::default().frequency(1.kHz().into()),
        )?;
        let driver = LedcDriver::new(channel, timer_driver, pin)?;
        let max_duty = driver.get_max_duty();

        let mut bl = Self {
            driver,
            max_duty,
            level: BrightLevel::Full,
            last_activity: Instant::now(),
            was_touched: false,
        };
        bl.set_level(BrightLevel::Full)?;
        Ok(bl)
    }

    pub fn tick(&mut self, touched: bool) -> Result<()> {
        let now = Instant::now();

        if touched && !self.was_touched {
            self.on_tap(now)?;
        }
        self.was_touched = touched;

        let idle = now.duration_since(self.last_activity);

        match self.level {
            BrightLevel::Full if idle >= DIM_TIMEOUT => {
                self.set_level(BrightLevel::Dim)?;
            }
            BrightLevel::Dim if idle >= OFF_TIMEOUT => {
                self.set_level(BrightLevel::Off)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn on_tap(&mut self, now: Instant) -> Result<()> {
        self.set_level(BrightLevel::Full)?;
        self.last_activity = now;
        Ok(())
    }

    fn set_level(&mut self, level: BrightLevel) -> Result<()> {
        self.level = level;
        let pct = match level {
            BrightLevel::Full => DUTY_FULL,
            BrightLevel::Dim => DUTY_DIM,
            BrightLevel::Off => DUTY_OFF,
        };
        let duty = self.max_duty * pct as u32 / 100;
        self.driver.set_duty(duty)?;
        Ok(())
    }
}
