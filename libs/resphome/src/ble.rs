#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleProxyMode {
    Disabled,
    Passive,
    Active,
}

impl BleProxyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Passive => "passive",
            Self::Active => "active",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "passive" => Self::Passive,
            _ => Self::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BleScanParams {
    pub interval_ms: u32,
    pub window_ms: u32,
    pub active: bool,
}

impl Default for BleScanParams {
    fn default() -> Self {
        Self {
            interval_ms: 1100,
            window_ms: 1100,
            active: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BleConfig {
    pub scan_params: BleScanParams,
    pub proxy_mode: BleProxyMode,
}

impl BleConfig {
    pub fn proxy_active() -> Self {
        Self {
            scan_params: BleScanParams::default(),
            proxy_mode: BleProxyMode::Active,
        }
    }

    pub fn proxy_passive() -> Self {
        Self {
            scan_params: BleScanParams::default(),
            proxy_mode: BleProxyMode::Passive,
        }
    }

    pub fn scanner_only() -> Self {
        Self {
            scan_params: BleScanParams::default(),
            proxy_mode: BleProxyMode::Disabled,
        }
    }

    pub fn with_scan_params(mut self, interval_ms: u32, window_ms: u32, active: bool) -> Self {
        self.scan_params = BleScanParams {
            interval_ms,
            window_ms,
            active,
        };
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BleDevice {
    pub mac_address: [u8; 6],
    pub rssi: i32,
    pub name: Option<String>,
    pub service_uuids: Vec<String>,
    pub service_data: Vec<(String, Vec<u8>)>,
    pub manufacturer_data: Vec<(u16, Vec<u8>)>,
}

impl BleDevice {
    pub fn new(mac: [u8; 6], rssi: i32) -> Self {
        Self {
            mac_address: mac,
            rssi,
            name: None,
            service_uuids: Vec::new(),
            service_data: Vec::new(),
            manufacturer_data: Vec::new(),
        }
    }

    pub fn mac_string(&self) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.mac_address[0], self.mac_address[1], self.mac_address[2],
            self.mac_address[3], self.mac_address[4], self.mac_address[5],
        )
    }

    pub fn from_mac_str(mac: &str) -> Option<[u8; 6]> {
        let parts: Vec<&str> = mac.split(':').collect();
        if parts.len() != 6 {
            return None;
        }
        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16).ok()?;
        }
        Some(bytes)
    }
}

pub trait BleScanner: Send + Sync {
    fn on_device_found(&self, device: &BleDevice);
    fn on_scan_complete(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ble_proxy_mode_roundtrip() {
        let modes = [BleProxyMode::Disabled, BleProxyMode::Passive, BleProxyMode::Active];
        for mode in &modes {
            assert_eq!(BleProxyMode::from_str(mode.as_str()), *mode);
        }
    }

    #[test]
    fn test_ble_proxy_mode_unknown() {
        assert_eq!(BleProxyMode::from_str("foobar"), BleProxyMode::Disabled);
    }

    #[test]
    fn test_ble_scan_params_default() {
        let params = BleScanParams::default();
        assert_eq!(params.interval_ms, 1100);
        assert_eq!(params.window_ms, 1100);
        assert!(params.active);
    }

    #[test]
    fn test_ble_config_proxy_active() {
        let config = BleConfig::proxy_active();
        assert_eq!(config.proxy_mode, BleProxyMode::Active);
        assert_eq!(config.scan_params.interval_ms, 1100);
    }

    #[test]
    fn test_ble_config_proxy_passive() {
        let config = BleConfig::proxy_passive();
        assert_eq!(config.proxy_mode, BleProxyMode::Passive);
    }

    #[test]
    fn test_ble_config_scanner_only() {
        let config = BleConfig::scanner_only();
        assert_eq!(config.proxy_mode, BleProxyMode::Disabled);
    }

    #[test]
    fn test_ble_config_custom_scan_params() {
        let config = BleConfig::proxy_active()
            .with_scan_params(500, 250, false);
        assert_eq!(config.scan_params.interval_ms, 500);
        assert_eq!(config.scan_params.window_ms, 250);
        assert!(!config.scan_params.active);
    }

    #[test]
    fn test_ble_device_mac_string() {
        let device = BleDevice::new([0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF], -65);
        assert_eq!(device.mac_string(), "A4:C1:38:92:48:AF");
    }

    #[test]
    fn test_ble_device_from_mac_str() {
        let mac = BleDevice::from_mac_str("A4:C1:38:92:48:AF").unwrap();
        assert_eq!(mac, [0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF]);
    }

    #[test]
    fn test_ble_device_from_mac_str_lowercase() {
        let mac = BleDevice::from_mac_str("a4:c1:38:92:48:af").unwrap();
        assert_eq!(mac, [0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF]);
    }

    #[test]
    fn test_ble_device_from_mac_str_invalid() {
        assert!(BleDevice::from_mac_str("invalid").is_none());
        assert!(BleDevice::from_mac_str("AA:BB:CC").is_none());
        assert!(BleDevice::from_mac_str("AA:BB:CC:DD:EE:GG").is_none());
    }

    #[test]
    fn test_ble_device_fields() {
        let mut device = BleDevice::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66], -80);
        device.name = Some("Sensor".to_string());
        device.service_uuids.push("0000181a-0000-1000-8000-00805f9b34fb".to_string());

        assert_eq!(device.rssi, -80);
        assert_eq!(device.name.as_deref(), Some("Sensor"));
        assert_eq!(device.service_uuids.len(), 1);
    }

    #[test]
    fn test_ble_device_mac_roundtrip() {
        let original = "A4:C1:38:92:48:AF";
        let bytes = BleDevice::from_mac_str(original).unwrap();
        let device = BleDevice::new(bytes, -50);
        assert_eq!(device.mac_string(), original);
    }
}
