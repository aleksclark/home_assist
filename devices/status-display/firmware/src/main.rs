mod ha;
mod ui;

use std::thread;
use std::time::Duration;

use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::config::Config as SpiConfig;
use esp_idf_hal::spi::{SpiDeviceDriver, SpiDriverConfig};
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};

use display_interface_spi::SPIInterface;
use mipidsi::models::ILI9341Rgb565;
use mipidsi::options::Orientation;
use mipidsi::Builder;

const WIFI_SSID: &str = "ClarkUltra";
const WIFI_PASS: &str = "deadbeef00";
const POLL_INTERVAL: Duration = Duration::from_secs(30);

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Status Display starting...");

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    // -- Display --
    let dc = PinDriver::output(peripherals.pins.gpio2)?;
    let mut backlight = PinDriver::output(peripherals.pins.gpio21)?;

    let spi = SpiDeviceDriver::new_single(
        peripherals.spi2,
        peripherals.pins.gpio14,
        peripherals.pins.gpio13,
        Option::<esp_idf_hal::gpio::AnyIOPin>::None,
        Some(peripherals.pins.gpio15),
        &SpiDriverConfig::new(),
        &SpiConfig::new().baudrate(26.MHz().into()).data_mode(embedded_hal::spi::MODE_0),
    )?;

    let mut display = Builder::new(ILI9341Rgb565, SPIInterface::new(spi, dc))
        .orientation(Orientation::new().rotate(mipidsi::options::Rotation::Deg90))
        .init(&mut Ets)
        .map_err(|e| anyhow::anyhow!("Display init: {:?}", e))?;

    backlight.set_high()?;
    log::info!("Display initialized");

    display.clear(Rgb565::new(2, 4, 2)).map_err(|_| anyhow::anyhow!("clear"))?;

    let boot_ui = ui::Dashboard::new(String::new());
    boot_ui.draw_boot_status(&mut display, "Connecting to WiFi...", None)?;

    // -- WiFi --
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs_partition.clone()))?,
        sysloop,
    )?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASS.try_into().unwrap(),
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip_str = format!("{}", wifi.wifi().sta_netif().get_ip_info()?.ip);
    log::info!("WiFi connected! IP: {}", ip_str);

    // -- HA registration --
    let mut dashboard = ui::Dashboard::new(ip_str.clone());
    dashboard.draw_boot_status(&mut display, "Connecting to HA...", Some(&ip_str))?;

    let mut nvs = ha::open_nvs(nvs_partition)?;
    let webhook_id = ha::register(&mut nvs)?;
    log::info!("Using webhook_id: {}", webhook_id);

    dashboard.draw_boot_status(&mut display, "Connected to HA!", Some(&ip_str))?;
    thread::sleep(Duration::from_secs(1));

    // -- Poll loop --
    loop {
        match ha::fetch_dashboard_data(&webhook_id) {
            Ok(data) => {
                log::info!("Dashboard data fetched");
                if let Err(e) = dashboard.update(&mut display, &data) {
                    log::error!("Draw error: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("Fetch error: {:?}", e);
                let _ = dashboard.draw_error(&mut display, &e);
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
}
