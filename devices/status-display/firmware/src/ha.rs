use std::time::Duration;

use anyhow::{bail, Result};
use embedded_svc::http::client::Client as HttpClient;
use embedded_svc::http::Method;
use embedded_svc::io::{Read, Write};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use serde::Deserialize;

const HA_HOST: &str = "192.168.0.3";
const HA_PORT: u16 = 8123;

const HA_PROVISION_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkMGNhYjE4ZDU0YzY0YTM1YTc1OGY5Njc1MmQxZDkzNSIsImlhdCI6MTc3MjkxMTEwOCwiZXhwIjoyMDg4MjcxMTA4fQ.D7xMKRFjoOUUIlSCb2zGX9v6IIP1agK30M-ABohkZS8";

const NVS_NAMESPACE: &str = "ha_app";
const NVS_KEY_WEBHOOK: &str = "webhook_id";

const ENTITIES: &[(&str, &str, TemplateKind)] = &[
    ("kitchen_temp",      "sensor.kitchen_temp_temperature",       TemplateKind::State),
    ("kitchen_humidity",  "sensor.kitchen_temp_humidity",           TemplateKind::State),
    ("bedroom_temp",      "sensor.atc_a0c6_temperature",           TemplateKind::State),
    ("bedroom_humidity",  "sensor.atc_a0c6_humidity",              TemplateKind::State),
    ("della_state",       "climate.della_mini_split",               TemplateKind::State),
    ("della_current_temp","climate.della_mini_split",               TemplateKind::Attr("current_temperature")),
    ("della_target_temp", "climate.della_mini_split",               TemplateKind::Attr("temperature")),
    ("della_fan",         "climate.della_mini_split",               TemplateKind::Attr("fan_mode")),
    ("ecobee_state",      "climate.my_ecobee",                     TemplateKind::State),
    ("ecobee_temp",       "sensor.my_ecobee_current_temperature",  TemplateKind::State),
    ("ecobee_humidity",   "sensor.my_ecobee_current_humidity",     TemplateKind::State),
];

#[derive(Clone, Copy)]
enum TemplateKind {
    State,
    Attr(&'static str),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DashboardData {
    pub kitchen_temp: Option<f32>,
    pub kitchen_humidity: Option<f32>,
    pub bedroom_temp: Option<f32>,
    pub bedroom_humidity: Option<f32>,
    pub della_mode: Option<String>,
    pub della_current_temp: Option<f32>,
    pub della_target_temp: Option<f32>,
    pub della_fan: Option<String>,
    pub ecobee_mode: Option<String>,
    pub ecobee_temp: Option<f32>,
    pub ecobee_humidity: Option<f32>,
}

#[derive(Deserialize)]
struct RegistrationResponse {
    webhook_id: String,
}

pub fn load_webhook_id(nvs: &EspNvs<NvsDefault>) -> Option<String> {
    let mut buf = [0u8; 128];
    match nvs.get_str(NVS_KEY_WEBHOOK, &mut buf) {
        Ok(Some(s)) => {
            let s = s.trim_end_matches('\0');
            if s.is_empty() { None } else { Some(s.to_string()) }
        }
        _ => None,
    }
}

pub fn save_webhook_id(nvs: &mut EspNvs<NvsDefault>, webhook_id: &str) -> Result<()> {
    nvs.set_str(NVS_KEY_WEBHOOK, webhook_id)?;
    Ok(())
}

pub fn open_nvs(partition: esp_idf_svc::nvs::EspDefaultNvsPartition) -> Result<EspNvs<NvsDefault>> {
    Ok(EspNvs::<NvsDefault>::new(partition, NVS_NAMESPACE, true)?)
}

pub fn register(nvs: &mut EspNvs<NvsDefault>) -> Result<String> {
    if let Some(id) = load_webhook_id(nvs) {
        log::info!("Found stored webhook_id: {}", id);
        return Ok(id);
    }

    log::info!("No webhook_id found, registering with HA...");
    let id = register_device()?;
    save_webhook_id(nvs, &id)?;
    log::info!("Registered! webhook_id: {}", id);
    Ok(id)
}

pub fn fetch_dashboard_data(webhook_id: &str) -> Result<DashboardData> {
    let mut templates = serde_json::Map::new();
    for &(key, entity, kind) in ENTITIES {
        let tpl = match kind {
            TemplateKind::State => format!("{{{{ states('{}') }}}}", entity),
            TemplateKind::Attr(attr) => format!("{{{{ state_attr('{}', '{}') }}}}", entity, attr),
        };
        let mut entry = serde_json::Map::new();
        entry.insert("template".into(), serde_json::Value::String(tpl));
        templates.insert(key.into(), serde_json::Value::Object(entry));
    }

    let body = serde_json::json!({
        "type": "render_template",
        "data": templates,
    });

    let vals = webhook_post(webhook_id, &body)?;
    log::info!("Template response: {}", vals);

    let mut data = DashboardData::default();
    data.kitchen_temp = parse_f32(vals.get("kitchen_temp"));
    data.kitchen_humidity = parse_f32(vals.get("kitchen_humidity"));
    data.bedroom_temp = parse_f32(vals.get("bedroom_temp"));
    data.bedroom_humidity = parse_f32(vals.get("bedroom_humidity"));
    data.della_current_temp = parse_f32(vals.get("della_current_temp"));
    data.della_target_temp = parse_f32(vals.get("della_target_temp"));
    data.della_mode = parse_string(vals.get("della_state"));
    data.della_fan = parse_string(vals.get("della_fan"));
    data.ecobee_mode = parse_string(vals.get("ecobee_state"));
    data.ecobee_temp = parse_f32(vals.get("ecobee_temp"));
    data.ecobee_humidity = parse_f32(vals.get("ecobee_humidity"));
    Ok(data)
}

fn http_client() -> Result<HttpClient<EspHttpConnection>> {
    let config = HttpConfig {
        buffer_size: Some(2048),
        buffer_size_tx: Some(1024),
        timeout: Some(Duration::from_secs(10)),
        ..Default::default()
    };
    Ok(HttpClient::wrap(EspHttpConnection::new(&config)?))
}

fn register_device() -> Result<String> {
    if HA_PROVISION_TOKEN == "YOUR_HA_TOKEN_HERE" {
        bail!(
            "HA_PROVISION_TOKEN not set. Generate a long-lived access token in \
             Home Assistant (Profile -> Long-Lived Access Tokens) and set it in main.rs."
        );
    }

    let url = format!("http://{}:{}/api/mobile_app/registrations", HA_HOST, HA_PORT);
    let payload = serde_json::json!({
        "device_id": "esp32_status_display_001",
        "app_id": "esp32_status_display",
        "app_name": "Status Display",
        "app_version": "0.1.0",
        "device_name": "Status Display",
        "manufacturer": "Espressif",
        "model": "ESP32-2432S028",
        "os_name": "esp-idf",
        "os_version": "5.3",
        "supports_encryption": false
    });

    let auth_value = format!("Bearer {}", HA_PROVISION_TOKEN);
    let resp_body = http_post_json(&url, &payload, Some(&auth_value))?;
    let reg: RegistrationResponse = serde_json::from_str(&resp_body)?;
    Ok(reg.webhook_id)
}

fn webhook_post(webhook_id: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
    let url = format!("http://{}:{}/api/webhook/{}", HA_HOST, HA_PORT, webhook_id);
    let resp_body = http_post_json(&url, body, None)?;
    let status_code_marker = resp_body.trim();
    if status_code_marker.is_empty() {
        bail!("Empty response from webhook");
    }
    Ok(serde_json::from_str(status_code_marker)?)
}

fn http_post_json(url: &str, body: &serde_json::Value, auth: Option<&str>) -> Result<String> {
    let body_str = body.to_string();
    let body_bytes = body_str.as_bytes();
    let content_len = body_bytes.len().to_string();

    let mut header_vec: Vec<(&str, &str)> = vec![
        ("Content-Type", "application/json"),
        ("Content-Length", content_len.as_str()),
    ];
    let auth_owned;
    if let Some(a) = auth {
        auth_owned = a.to_string();
        header_vec.push(("Authorization", &auth_owned));
    }

    let mut client = http_client()?;
    let mut request = client.request(Method::Post, url, &header_vec)?;
    request.write_all(body_bytes)?;
    let mut response = request.submit()?;

    let status = response.status();
    let resp_body = read_body(&mut response)?;

    if status == 410 {
        bail!("Webhook expired (HTTP 410) — device must re-register");
    }
    if status < 200 || status >= 300 {
        bail!("HTTP {} from {}: {}", status, url, resp_body);
    }
    Ok(resp_body)
}

fn read_body<R: Read>(response: &mut R) -> Result<String> {
    let mut buf = [0u8; 2048];
    let mut body = String::new();
    loop {
        match response.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(_) => break,
        }
    }
    Ok(body)
}

fn parse_f32(v: Option<&serde_json::Value>) -> Option<f32> {
    v.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_f64().map(|f| f as f32),
        serde_json::Value::String(s) if s != "unavailable" && s != "unknown" => s.parse().ok(),
        _ => None,
    })
}

fn parse_string(v: Option<&serde_json::Value>) -> Option<String> {
    v.and_then(|v| match v {
        serde_json::Value::String(s) if s != "unavailable" && s != "unknown" => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}
