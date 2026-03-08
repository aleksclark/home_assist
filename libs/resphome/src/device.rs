#[derive(Debug, Clone, PartialEq)]
pub struct DeviceConfig {
    pub name: String,
    pub friendly_name: String,
    pub mac_address: String,
    pub model: String,
    pub manufacturer: String,
    pub sw_version: String,
    pub compilation_time: String,
    pub project_name: String,
    pub project_version: String,
    pub has_deep_sleep: bool,
}

impl DeviceConfig {
    pub fn new(name: &str, friendly_name: &str) -> Self {
        Self {
            name: name.to_string(),
            friendly_name: friendly_name.to_string(),
            mac_address: String::new(),
            model: String::new(),
            manufacturer: "Espressif".to_string(),
            sw_version: String::new(),
            compilation_time: String::new(),
            project_name: String::new(),
            project_version: String::new(),
            has_deep_sleep: false,
        }
    }

    pub fn with_mac(mut self, mac: &str) -> Self {
        self.mac_address = mac.to_string();
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_manufacturer(mut self, manufacturer: &str) -> Self {
        self.manufacturer = manufacturer.to_string();
        self
    }

    pub fn with_sw_version(mut self, version: &str) -> Self {
        self.sw_version = version.to_string();
        self
    }

    pub fn with_project(mut self, name: &str, version: &str) -> Self {
        self.project_name = name.to_string();
        self.project_version = version.to_string();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceInfo {
    pub uses_password: bool,
    pub name: String,
    pub mac_address: String,
    pub esphome_version: String,
    pub compilation_time: String,
    pub model: String,
    pub has_deep_sleep: bool,
    pub project_name: String,
    pub project_version: String,
    pub webserver_port: u32,
    pub manufacturer: String,
    pub friendly_name: String,
}

impl DeviceInfo {
    pub fn from_config(config: &DeviceConfig, esphome_version: &str) -> Self {
        Self {
            uses_password: false,
            name: config.name.clone(),
            mac_address: config.mac_address.clone(),
            esphome_version: esphome_version.to_string(),
            compilation_time: config.compilation_time.clone(),
            model: config.model.clone(),
            has_deep_sleep: config.has_deep_sleep,
            project_name: config.project_name.clone(),
            project_version: config.project_version.clone(),
            webserver_port: 0,
            manufacturer: config.manufacturer.clone(),
            friendly_name: config.friendly_name.clone(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        use crate::proto::*;
        let mut buf = Vec::new();
        encode_field_bool(1, self.uses_password, &mut buf);
        encode_field_string(2, &self.name, &mut buf);
        encode_field_string(3, &self.mac_address, &mut buf);
        encode_field_string(4, &self.esphome_version, &mut buf);
        encode_field_string(5, &self.compilation_time, &mut buf);
        encode_field_string(6, &self.model, &mut buf);
        encode_field_bool(7, self.has_deep_sleep, &mut buf);
        encode_field_string(8, &self.project_name, &mut buf);
        encode_field_string(9, &self.project_version, &mut buf);
        encode_field_varint(10, self.webserver_port as u64, &mut buf);
        encode_field_string(12, &self.manufacturer, &mut buf);
        encode_field_string(13, &self.friendly_name, &mut buf);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::FieldIter;

    #[test]
    fn test_device_config_builder() {
        let config = DeviceConfig::new("test-dev", "Test Device")
            .with_mac("AA:BB:CC:DD:EE:FF")
            .with_model("ESP32-WROOM-32")
            .with_manufacturer("Espressif")
            .with_sw_version("0.1.0")
            .with_project("resphome", "0.1.0");

        assert_eq!(config.name, "test-dev");
        assert_eq!(config.friendly_name, "Test Device");
        assert_eq!(config.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(config.model, "ESP32-WROOM-32");
        assert_eq!(config.project_name, "resphome");
        assert_eq!(config.project_version, "0.1.0");
        assert!(!config.has_deep_sleep);
    }

    #[test]
    fn test_device_info_from_config() {
        let config = DeviceConfig::new("scanner", "BLE Scanner")
            .with_mac("11:22:33:44:55:66")
            .with_model("ESP32-WROOM-32");

        let info = DeviceInfo::from_config(&config, "2024.1.0");
        assert_eq!(info.name, "scanner");
        assert_eq!(info.friendly_name, "BLE Scanner");
        assert_eq!(info.mac_address, "11:22:33:44:55:66");
        assert_eq!(info.esphome_version, "2024.1.0");
        assert!(!info.uses_password);
    }

    #[test]
    fn test_device_info_encode_decode() {
        let config = DeviceConfig::new("test", "Test")
            .with_mac("AA:BB:CC:DD:EE:FF")
            .with_model("ESP32");

        let info = DeviceInfo::from_config(&config, "2024.1.0");
        let encoded = info.encode();

        let mut name = "";
        let mut mac = "";
        let mut version = "";
        let mut model = "";
        let mut friendly = "";

        for (field, value) in FieldIter::new(&encoded) {
            match field {
                2 => name = value.as_str(),
                3 => mac = value.as_str(),
                4 => version = value.as_str(),
                6 => model = value.as_str(),
                13 => friendly = value.as_str(),
                _ => {}
            }
        }

        assert_eq!(name, "test");
        assert_eq!(mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(version, "2024.1.0");
        assert_eq!(model, "ESP32");
        assert_eq!(friendly, "Test");
    }
}
