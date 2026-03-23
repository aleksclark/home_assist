mod api;
mod backlight;
mod ota;
mod proto;
mod state;
mod touch;
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
use esp_idf_svc::sntp::{EspSntp, SyncStatus};
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};

use display_interface_spi::SPIInterface;
use mipidsi::models::ILI9341Rgb565;
use mipidsi::options::Orientation;
use mipidsi::Builder;

const WIFI_SSID: &str = "ClarkUltra";
const WIFI_PASS: &str = "deadbeef00";
const API_PORT: u16 = 6053;
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Status Display starting...");

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    // -- Display (SPI2) --
    let dc = PinDriver::output(peripherals.pins.gpio2)?;

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
        .orientation(Orientation::new().rotate(mipidsi::options::Rotation::Deg270))
        .init(&mut Ets)
        .map_err(|e| anyhow::anyhow!("Display init: {:?}", e))?;

    // -- Backlight (LEDC PWM) --
    let mut bl = backlight::Backlight::new(
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.pins.gpio21,
    )?;

    log::info!("Display initialized");

    display.clear(Rgb565::new(2, 4, 2)).map_err(|_| anyhow::anyhow!("clear"))?;

    let mut dashboard = ui::Dashboard::new(String::new());
    dashboard.draw_boot_status(&mut display, "Connecting to WiFi...", None)?;

    // -- Touch (SPI3 / XPT2046) --
    let touch = touch::Touch::new(
        peripherals.spi3,
        peripherals.pins.gpio25,
        peripherals.pins.gpio32,
        peripherals.pins.gpio39,
        peripherals.pins.gpio33,
        peripherals.pins.gpio36,
    )?;
    log::info!("Touch initialized");

    // -- WiFi --
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs_partition))?,
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

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    let ip_str = format!("{}", ip_info.ip);
    log::info!("WiFi connected! IP: {}", ip_str);

    dashboard = ui::Dashboard::new(ip_str.clone());
    dashboard.draw_boot_status(&mut display, "Syncing time...", Some(&ip_str))?;

    // -- SNTP --
    std::env::set_var("TZ", "CST6CDT,M3.2.0,M11.1.0");
    let _sntp = EspSntp::new_default()?;
    for _ in 0..40 {
        if _sntp.get_sync_status() == SyncStatus::Completed {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    log::info!("SNTP sync status: {:?}", _sntp.get_sync_status());

    dashboard.draw_boot_status(&mut display, "Starting API server...", Some(&ip_str))?;

    // -- MAC address --
    let mac_bytes = wifi.wifi().sta_netif().get_mac()?;
    let mac_str = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac_bytes[0], mac_bytes[1], mac_bytes[2],
        mac_bytes[3], mac_bytes[4], mac_bytes[5],
    );

    // -- Shared state --
    let shared_state = state::new_shared();

    // -- API server thread --
    let api_state = shared_state.clone();
    let api_mac = mac_str.clone();
    thread::Builder::new()
        .name("api-server".into())
        .stack_size(65536)
        .spawn(move || {
            api::start_server(api_state, api_mac, API_PORT);
        })?;

    dashboard.draw_boot_status(&mut display, "Waiting for HA...", Some(&ip_str))?;
    log::info!("Ready. Waiting for Home Assistant to connect.");

    // -- Display refresh loop --
    loop {
        if let Some(stream) = api::take_ota_stream() {
            log::info!("OTA: handling on main thread");
            if let Err(e) = ota::handle_update(stream) {
                log::error!("OTA failed: {:?}", e);
            }
        }

        bl.tick(touch.is_touched())?;

        if let Err(e) = dashboard.update(&mut display, &shared_state) {
            log::error!("Draw error: {:?}", e);
        }
        thread::sleep(REFRESH_INTERVAL);
    }
}
