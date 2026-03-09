use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};

use resphome::api::{ApiConfig, ApiServer, ClientHandler, ESPHOME_VERSION};
use resphome::device::{DeviceConfig, DeviceInfo};
use resphome::entity::{
    BinarySensorEntity, Entity, EntityState, SensorClass, SensorEntity, TextSensorEntity,
};
use resphome::wifi::WifiConfig;

const WIFI_SSID: &str = "ClarkUltra";
const WIFI_PASS: &str = "deadbeef00";

struct BleProxyHandler {
    device_info: DeviceInfo,
}

impl BleProxyHandler {
    fn new(mac: &str) -> Self {
        let config = DeviceConfig::new("resphome-test", "RESPHome Test")
            .with_mac(mac)
            .with_model("ESP32-WROOM-32")
            .with_manufacturer("Espressif")
            .with_sw_version("0.1.0")
            .with_project("resphome.resphome-test", "0.1.0");

        Self {
            device_info: DeviceInfo::from_config(&config, ESPHOME_VERSION),
        }
    }
}

impl ClientHandler for BleProxyHandler {
    fn device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }

    fn list_entities(&self) -> Vec<Entity> {
        vec![
            Entity::Sensor(
                SensorEntity::new(0x0001, "wifi_signal", "WiFi Signal")
                    .with_unit("dBm")
                    .with_device_class(SensorClass::SignalStrength)
                    .with_accuracy(0)
                    .with_icon("mdi:wifi"),
            ),
            Entity::BinarySensor(
                BinarySensorEntity::new(0x0002, "status", "Status").as_status_sensor(),
            ),
            Entity::TextSensor(
                TextSensorEntity::new(0x0003, "version", "Firmware Version")
                    .with_icon("mdi:tag"),
            ),
        ]
    }

    fn get_states(&self) -> Vec<EntityState> {
        vec![
            EntityState::Sensor {
                key: 0x0001,
                value: -55.0,
                missing: false,
            },
            EntityState::BinarySensor {
                key: 0x0002,
                state: true,
                missing: false,
            },
            EntityState::TextSensor {
                key: 0x0003,
                value: "0.1.0".to_string(),
                missing: false,
            },
        ]
    }

    fn on_ha_state(&self, entity_id: &str, attribute: &str, state: &str) {
        log::info!("HA state update: {} {} = {}", entity_id, attribute, state);
    }

    fn on_select_command(&self, key: u32, value: &str) {
        log::info!("Select command: key=0x{:x} value={}", key, value);
    }

    fn ha_subscriptions(&self) -> Vec<(String, String)> {
        vec![]
    }
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("RESPHome test firmware starting...");

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    let _wifi_config = WifiConfig::stable_preset(WIFI_SSID, WIFI_PASS);

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
    log::info!("WiFi connected! IP: {}", ip_info.ip);

    let mac_bytes = wifi.wifi().sta_netif().get_mac()?;
    let mac_str = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac_bytes[0], mac_bytes[1], mac_bytes[2],
        mac_bytes[3], mac_bytes[4], mac_bytes[5],
    );
    log::info!("MAC: {}", mac_str);

    let handler: Arc<dyn ClientHandler> = Arc::new(BleProxyHandler::new(&mac_str));
    let api_config = ApiConfig::new(6053);

    let api_handler = handler.clone();
    thread::Builder::new()
        .name("api-server".into())
        .stack_size(8192)
        .spawn(move || {
            let server = ApiServer::new(api_config, api_handler);
            if let Err(e) = server.run() {
                log::error!("API server error: {:?}", e);
            }
        })?;

    log::info!("ESPHome API server started on port 6053");
    log::info!("Ready for Home Assistant connection.");

    loop {
        thread::sleep(Duration::from_secs(30));
        log::info!("Heartbeat - device running");
    }
}
