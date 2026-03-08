use crate::entity::SensorClass;

#[derive(Debug, Clone, PartialEq)]
pub struct SensorReading {
    pub value: f64,
    pub unit: String,
    pub device_class: SensorClass,
    pub accuracy_decimals: i32,
}

impl SensorReading {
    pub fn temperature(value: f64) -> Self {
        Self {
            value,
            unit: "°C".to_string(),
            device_class: SensorClass::Temperature,
            accuracy_decimals: 1,
        }
    }

    pub fn humidity(value: f64) -> Self {
        Self {
            value,
            unit: "%".to_string(),
            device_class: SensorClass::Humidity,
            accuracy_decimals: 0,
        }
    }

    pub fn battery(value: f64) -> Self {
        Self {
            value,
            unit: "%".to_string(),
            device_class: SensorClass::Battery,
            accuracy_decimals: 0,
        }
    }

    pub fn signal_strength(value: f64) -> Self {
        Self {
            value,
            unit: "dBm".to_string(),
            device_class: SensorClass::SignalStrength,
            accuracy_decimals: 0,
        }
    }

    pub fn custom(value: f64, unit: &str, class: SensorClass, decimals: i32) -> Self {
        Self {
            value,
            unit: unit.to_string(),
            device_class: class,
            accuracy_decimals: decimals,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensorPlatform {
    AtcMiThermometer,
    Generic,
}

impl SensorPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AtcMiThermometer => "atc_mithermometer",
            Self::Generic => "generic",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "atc_mithermometer" => Self::AtcMiThermometer,
            _ => Self::Generic,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtcMiThermometerConfig {
    pub mac_address: [u8; 6],
    pub name_prefix: String,
}

impl AtcMiThermometerConfig {
    pub fn new(mac: [u8; 6], name_prefix: &str) -> Self {
        Self {
            mac_address: mac,
            name_prefix: name_prefix.to_string(),
        }
    }

    pub fn temperature_name(&self) -> String {
        format!("{} Temperature", self.name_prefix)
    }

    pub fn humidity_name(&self) -> String {
        format!("{} Humidity", self.name_prefix)
    }

    pub fn battery_name(&self) -> String {
        format!("{} Battery", self.name_prefix)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtcMiThermometerData {
    pub mac_address: [u8; 6],
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub battery_percent: Option<f64>,
    pub battery_mv: Option<f64>,
}

impl AtcMiThermometerData {
    pub fn parse_advertisement(mac: [u8; 6], data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        if data[0..6] != mac {
            return None;
        }

        let temp_raw = i16::from_be_bytes([data[6], data[7]]);
        let temperature = temp_raw as f64 / 10.0;

        let humidity = data[8] as f64;

        let battery_percent = data[9] as f64;

        let battery_mv = u16::from_be_bytes([data[10], data[11]]) as f64;

        Some(Self {
            mac_address: mac,
            temperature: Some(temperature),
            humidity: Some(humidity),
            battery_percent: Some(battery_percent),
            battery_mv: Some(battery_mv),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ble::BleDevice;

    #[test]
    fn test_sensor_reading_temperature() {
        let reading = SensorReading::temperature(22.5);
        assert!((reading.value - 22.5).abs() < 0.001);
        assert_eq!(reading.unit, "°C");
        assert_eq!(reading.device_class, SensorClass::Temperature);
        assert_eq!(reading.accuracy_decimals, 1);
    }

    #[test]
    fn test_sensor_reading_humidity() {
        let reading = SensorReading::humidity(65.0);
        assert!((reading.value - 65.0).abs() < 0.001);
        assert_eq!(reading.unit, "%");
        assert_eq!(reading.device_class, SensorClass::Humidity);
    }

    #[test]
    fn test_sensor_reading_battery() {
        let reading = SensorReading::battery(85.0);
        assert!((reading.value - 85.0).abs() < 0.001);
        assert_eq!(reading.unit, "%");
        assert_eq!(reading.device_class, SensorClass::Battery);
    }

    #[test]
    fn test_sensor_reading_signal_strength() {
        let reading = SensorReading::signal_strength(-65.0);
        assert!((reading.value - -65.0).abs() < 0.001);
        assert_eq!(reading.unit, "dBm");
        assert_eq!(reading.device_class, SensorClass::SignalStrength);
    }

    #[test]
    fn test_sensor_reading_custom() {
        let reading = SensorReading::custom(1013.25, "hPa", SensorClass::Pressure, 2);
        assert_eq!(reading.unit, "hPa");
        assert_eq!(reading.device_class, SensorClass::Pressure);
        assert_eq!(reading.accuracy_decimals, 2);
    }

    #[test]
    fn test_sensor_platform_roundtrip() {
        let platforms = [SensorPlatform::AtcMiThermometer, SensorPlatform::Generic];
        for p in &platforms {
            assert_eq!(SensorPlatform::from_str(p.as_str()), *p);
        }
    }

    #[test]
    fn test_sensor_platform_unknown() {
        assert_eq!(SensorPlatform::from_str("weird"), SensorPlatform::Generic);
    }

    #[test]
    fn test_atc_mi_thermometer_config() {
        let mac = BleDevice::from_mac_str("A4:C1:38:92:48:AF").unwrap();
        let config = AtcMiThermometerConfig::new(mac, "Room");
        assert_eq!(config.temperature_name(), "Room Temperature");
        assert_eq!(config.humidity_name(), "Room Humidity");
        assert_eq!(config.battery_name(), "Room Battery");
    }

    #[test]
    fn test_atc_mi_thermometer_parse() {
        let mac: [u8; 6] = [0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF];
        let mut data = Vec::new();
        data.extend_from_slice(&mac);
        data.extend_from_slice(&[0x00, 0xE1]); // temp = 22.5 (225 / 10)
        data.push(65); // humidity = 65%
        data.push(85); // battery = 85%
        data.extend_from_slice(&[0x0B, 0xF4]); // battery_mv = 3060

        let parsed = AtcMiThermometerData::parse_advertisement(mac, &data).unwrap();
        assert!((parsed.temperature.unwrap() - 22.5).abs() < 0.1);
        assert!((parsed.humidity.unwrap() - 65.0).abs() < 0.1);
        assert!((parsed.battery_percent.unwrap() - 85.0).abs() < 0.1);
        assert!((parsed.battery_mv.unwrap() - 3060.0).abs() < 0.1);
    }

    #[test]
    fn test_atc_mi_thermometer_parse_negative_temp() {
        let mac: [u8; 6] = [0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF];
        let mut data = Vec::new();
        data.extend_from_slice(&mac);
        data.extend_from_slice(&[0xFF, 0xCE]); // temp = -5.0 (-50 as i16 BE / 10)
        data.push(80);
        data.push(90);
        data.extend_from_slice(&[0x0C, 0x1C]);

        let parsed = AtcMiThermometerData::parse_advertisement(mac, &data).unwrap();
        assert!((parsed.temperature.unwrap() - (-5.0)).abs() < 0.1);
    }

    #[test]
    fn test_atc_mi_thermometer_parse_too_short() {
        let mac: [u8; 6] = [0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF];
        let data = [0u8; 5];
        assert!(AtcMiThermometerData::parse_advertisement(mac, &data).is_none());
    }

    #[test]
    fn test_atc_mi_thermometer_parse_wrong_mac() {
        let mac: [u8; 6] = [0xA4, 0xC1, 0x38, 0x92, 0x48, 0xAF];
        let wrong_mac: [u8; 6] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut data = Vec::new();
        data.extend_from_slice(&wrong_mac);
        data.extend_from_slice(&[0x00, 0xE1, 65, 85, 0x0B, 0xF4, 0x00]);

        assert!(AtcMiThermometerData::parse_advertisement(mac, &data).is_none());
    }
}
