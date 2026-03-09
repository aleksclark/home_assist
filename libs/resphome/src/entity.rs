use crate::proto::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorClass {
    None,
    Temperature,
    Humidity,
    Battery,
    Pressure,
    Power,
    Energy,
    Voltage,
    Current,
    SignalStrength,
    Illuminance,
}

impl SensorClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Temperature => "temperature",
            Self::Humidity => "humidity",
            Self::Battery => "battery",
            Self::Pressure => "pressure",
            Self::Power => "power",
            Self::Energy => "energy",
            Self::Voltage => "voltage",
            Self::Current => "current",
            Self::SignalStrength => "signal_strength",
            Self::Illuminance => "illuminance",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "temperature" => Self::Temperature,
            "humidity" => Self::Humidity,
            "battery" => Self::Battery,
            "pressure" => Self::Pressure,
            "power" => Self::Power,
            "energy" => Self::Energy,
            "voltage" => Self::Voltage,
            "current" => Self::Current,
            "signal_strength" => Self::SignalStrength,
            "illuminance" => Self::Illuminance,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorEntity {
    pub key: u32,
    pub object_id: String,
    pub name: String,
    pub icon: String,
    pub unit_of_measurement: String,
    pub accuracy_decimals: i32,
    pub device_class: SensorClass,
    pub state_class: u32,
    pub disabled_by_default: bool,
}

impl SensorEntity {
    pub fn new(key: u32, object_id: &str, name: &str) -> Self {
        Self {
            key,
            object_id: object_id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            unit_of_measurement: String::new(),
            accuracy_decimals: 0,
            device_class: SensorClass::None,
            state_class: 0,
            disabled_by_default: false,
        }
    }

    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit_of_measurement = unit.to_string();
        self
    }

    pub fn with_accuracy(mut self, decimals: i32) -> Self {
        self.accuracy_decimals = decimals;
        self
    }

    pub fn with_device_class(mut self, class: SensorClass) -> Self {
        self.device_class = class;
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = icon.to_string();
        self
    }

    pub fn encode_list_entry(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_field_string(1, &self.object_id, &mut buf);
        encode_field_fixed32(2, self.key, &mut buf);
        encode_field_string(3, &self.name, &mut buf);
        if !self.icon.is_empty() {
            encode_field_string(5, &self.icon, &mut buf);
        }
        encode_field_string(6, &self.unit_of_measurement, &mut buf);
        encode_field_varint(7, self.accuracy_decimals as u64, &mut buf);
        if self.disabled_by_default {
            encode_field_bool(9, true, &mut buf);
        }
        let dc = self.device_class.as_str();
        if !dc.is_empty() {
            encode_field_string(12, dc, &mut buf);
        }
        buf
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSensorEntity {
    pub key: u32,
    pub object_id: String,
    pub name: String,
    pub icon: String,
    pub disabled_by_default: bool,
}

impl TextSensorEntity {
    pub fn new(key: u32, object_id: &str, name: &str) -> Self {
        Self {
            key,
            object_id: object_id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            disabled_by_default: false,
        }
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = icon.to_string();
        self
    }

    pub fn encode_list_entry(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_field_string(1, &self.object_id, &mut buf);
        encode_field_fixed32(2, self.key, &mut buf);
        encode_field_string(3, &self.name, &mut buf);
        if !self.icon.is_empty() {
            encode_field_string(5, &self.icon, &mut buf);
        }
        if self.disabled_by_default {
            encode_field_bool(7, true, &mut buf);
        }
        buf
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinarySensorEntity {
    pub key: u32,
    pub object_id: String,
    pub name: String,
    pub icon: String,
    pub device_class: String,
    pub disabled_by_default: bool,
    pub is_status_binary_sensor: bool,
}

impl BinarySensorEntity {
    pub fn new(key: u32, object_id: &str, name: &str) -> Self {
        Self {
            key,
            object_id: object_id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            device_class: String::new(),
            disabled_by_default: false,
            is_status_binary_sensor: false,
        }
    }

    pub fn with_device_class(mut self, class: &str) -> Self {
        self.device_class = class.to_string();
        self
    }

    pub fn as_status_sensor(mut self) -> Self {
        self.is_status_binary_sensor = true;
        self.device_class = "connectivity".to_string();
        self
    }

    pub fn encode_list_entry(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_field_string(1, &self.object_id, &mut buf);
        encode_field_fixed32(2, self.key, &mut buf);
        encode_field_string(3, &self.name, &mut buf);
        if !self.icon.is_empty() {
            encode_field_string(5, &self.icon, &mut buf);
        }
        if !self.device_class.is_empty() {
            encode_field_string(6, &self.device_class, &mut buf);
        }
        encode_field_bool(7, self.is_status_binary_sensor, &mut buf);
        if self.disabled_by_default {
            encode_field_bool(8, true, &mut buf);
        }
        buf
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectEntity {
    pub key: u32,
    pub object_id: String,
    pub name: String,
    pub icon: String,
    pub options: Vec<String>,
    pub disabled_by_default: bool,
}

impl SelectEntity {
    pub fn new(key: u32, object_id: &str, name: &str, options: &[&str]) -> Self {
        Self {
            key,
            object_id: object_id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            options: options.iter().map(|s| s.to_string()).collect(),
            disabled_by_default: false,
        }
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = icon.to_string();
        self
    }

    pub fn encode_list_entry(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_field_string(1, &self.object_id, &mut buf);
        encode_field_fixed32(2, self.key, &mut buf);
        encode_field_string(3, &self.name, &mut buf);
        if !self.icon.is_empty() {
            encode_field_string(5, &self.icon, &mut buf);
        }
        for opt in &self.options {
            encode_field_string(6, opt, &mut buf);
        }
        buf
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entity {
    Sensor(SensorEntity),
    TextSensor(TextSensorEntity),
    BinarySensor(BinarySensorEntity),
    Select(SelectEntity),
}

impl Entity {
    pub fn key(&self) -> u32 {
        match self {
            Self::Sensor(e) => e.key,
            Self::TextSensor(e) => e.key,
            Self::BinarySensor(e) => e.key,
            Self::Select(e) => e.key,
        }
    }

    pub fn object_id(&self) -> &str {
        match self {
            Self::Sensor(e) => &e.object_id,
            Self::TextSensor(e) => &e.object_id,
            Self::BinarySensor(e) => &e.object_id,
            Self::Select(e) => &e.object_id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Sensor(e) => &e.name,
            Self::TextSensor(e) => &e.name,
            Self::BinarySensor(e) => &e.name,
            Self::Select(e) => &e.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    Sensor,
    TextSensor,
    BinarySensor,
    Select,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityState {
    Sensor { key: u32, value: f32, missing: bool },
    TextSensor { key: u32, value: String, missing: bool },
    BinarySensor { key: u32, state: bool, missing: bool },
    Select { key: u32, value: String, missing: bool },
}

impl EntityState {
    pub fn key(&self) -> u32 {
        match self {
            Self::Sensor { key, .. } => *key,
            Self::TextSensor { key, .. } => *key,
            Self::BinarySensor { key, .. } => *key,
            Self::Select { key, .. } => *key,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Self::Sensor { key, value, missing } => {
                encode_field_fixed32(1, *key, &mut buf);
                encode_field_float(2, *value, &mut buf);
                encode_field_bool(3, *missing, &mut buf);
            }
            Self::TextSensor { key, value, missing } => {
                encode_field_fixed32(1, *key, &mut buf);
                encode_field_string(2, value, &mut buf);
                encode_field_bool(3, *missing, &mut buf);
            }
            Self::BinarySensor { key, state, missing } => {
                encode_field_fixed32(1, *key, &mut buf);
                encode_field_bool(2, *state, &mut buf);
                encode_field_bool(3, *missing, &mut buf);
            }
            Self::Select { key, value, missing } => {
                encode_field_fixed32(1, *key, &mut buf);
                encode_field_string(2, value, &mut buf);
                encode_field_bool(3, *missing, &mut buf);
            }
        }
        buf
    }

    pub fn msg_type(&self) -> u32 {
        match self {
            Self::Sensor { .. } => 25,
            Self::TextSensor { .. } => 27,
            Self::BinarySensor { .. } => 21,
            Self::Select { .. } => 53,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_entity_builder() {
        let entity = SensorEntity::new(0x100, "temperature", "Temperature")
            .with_unit("°C")
            .with_accuracy(1)
            .with_device_class(SensorClass::Temperature)
            .with_icon("mdi:thermometer");

        assert_eq!(entity.key, 0x100);
        assert_eq!(entity.object_id, "temperature");
        assert_eq!(entity.name, "Temperature");
        assert_eq!(entity.unit_of_measurement, "°C");
        assert_eq!(entity.accuracy_decimals, 1);
        assert_eq!(entity.device_class, SensorClass::Temperature);
        assert_eq!(entity.icon, "mdi:thermometer");
    }

    #[test]
    fn test_sensor_entity_encode() {
        let entity = SensorEntity::new(0x100, "temp", "Temperature")
            .with_unit("°C")
            .with_accuracy(1);
        let encoded = entity.encode_list_entry();

        let mut object_id = "";
        let mut name = "";
        let mut unit = "";
        let mut key = 0u32;

        for (field, value) in FieldIter::new(&encoded) {
            match field {
                1 => object_id = value.as_str(),
                2 => key = value.as_u32(),
                3 => name = value.as_str(),
                6 => unit = value.as_str(),
                _ => {}
            }
        }

        assert_eq!(object_id, "temp");
        assert_eq!(key, 0x100);
        assert_eq!(name, "Temperature");
        assert_eq!(unit, "°C");
    }

    #[test]
    fn test_text_sensor_entity_encode() {
        let entity = TextSensorEntity::new(0x200, "status", "Status")
            .with_icon("mdi:information");
        let encoded = entity.encode_list_entry();

        let mut object_id = "";
        let mut name = "";
        let mut icon = "";
        let mut key = 0u32;

        for (field, value) in FieldIter::new(&encoded) {
            match field {
                1 => object_id = value.as_str(),
                2 => key = value.as_u32(),
                3 => name = value.as_str(),
                5 => icon = value.as_str(),
                _ => {}
            }
        }

        assert_eq!(object_id, "status");
        assert_eq!(key, 0x200);
        assert_eq!(name, "Status");
        assert_eq!(icon, "mdi:information");
    }

    #[test]
    fn test_binary_sensor_entity_encode() {
        let entity = BinarySensorEntity::new(0x300, "connectivity", "Status")
            .as_status_sensor();
        let encoded = entity.encode_list_entry();

        let mut object_id = "";
        let mut key = 0u32;
        let mut device_class = "";

        for (field, value) in FieldIter::new(&encoded) {
            match field {
                1 => object_id = value.as_str(),
                2 => key = value.as_u32(),
                6 => device_class = value.as_str(),
                _ => {}
            }
        }

        assert_eq!(object_id, "connectivity");
        assert_eq!(key, 0x300);
        assert_eq!(device_class, "connectivity");
    }

    #[test]
    fn test_select_entity_encode() {
        let entity = SelectEntity::new(0x400, "mode", "Mode", &["auto", "heat", "cool"])
            .with_icon("mdi:thermostat");
        let encoded = entity.encode_list_entry();

        let mut object_id = "";
        let mut key = 0u32;
        let mut options = Vec::new();

        for (field, value) in FieldIter::new(&encoded) {
            match field {
                1 => object_id = value.as_str(),
                2 => key = value.as_u32(),
                6 => options.push(value.as_str().to_string()),
                _ => {}
            }
        }

        assert_eq!(object_id, "mode");
        assert_eq!(key, 0x400);
        assert_eq!(options, vec!["auto", "heat", "cool"]);
    }

    #[test]
    fn test_entity_enum_accessors() {
        let s = Entity::Sensor(SensorEntity::new(1, "s1", "Sensor 1"));
        assert_eq!(s.key(), 1);
        assert_eq!(s.object_id(), "s1");
        assert_eq!(s.name(), "Sensor 1");

        let t = Entity::TextSensor(TextSensorEntity::new(2, "t1", "Text 1"));
        assert_eq!(t.key(), 2);
        assert_eq!(t.object_id(), "t1");

        let b = Entity::BinarySensor(BinarySensorEntity::new(3, "b1", "Binary 1"));
        assert_eq!(b.key(), 3);

        let sel = Entity::Select(SelectEntity::new(4, "sel1", "Select 1", &["a", "b"]));
        assert_eq!(sel.key(), 4);
    }

    #[test]
    fn test_entity_state_sensor_encode() {
        let state = EntityState::Sensor { key: 0x100, value: 22.5, missing: false };
        let encoded = state.encode();

        let mut key = 0u32;
        let mut val = 0.0f32;
        for (field, value) in FieldIter::new(&encoded) {
            match field {
                1 => key = value.as_u32(),
                2 => val = value.as_f32(),
                _ => {}
            }
        }

        assert_eq!(key, 0x100);
        assert!((val - 22.5).abs() < 0.01);
        assert_eq!(state.msg_type(), 25);
    }

    #[test]
    fn test_entity_state_text_sensor_encode() {
        let state = EntityState::TextSensor {
            key: 0x200,
            value: "active".to_string(),
            missing: false,
        };
        let encoded = state.encode();

        let mut key = 0u32;
        let mut val = "";
        for (field, value) in FieldIter::new(&encoded) {
            match field {
                1 => key = value.as_u32(),
                2 => val = value.as_str(),
                _ => {}
            }
        }

        assert_eq!(key, 0x200);
        assert_eq!(val, "active");
        assert_eq!(state.msg_type(), 27);
    }

    #[test]
    fn test_entity_state_binary_sensor_encode() {
        let state = EntityState::BinarySensor { key: 0x300, state: true, missing: false };
        assert_eq!(state.msg_type(), 21);
        assert_eq!(state.key(), 0x300);
    }

    #[test]
    fn test_entity_state_select_encode() {
        let state = EntityState::Select {
            key: 0x400,
            value: "heat".to_string(),
            missing: false,
        };
        let encoded = state.encode();
        assert_eq!(state.msg_type(), 53);

        let mut val = "";
        for (field, value) in FieldIter::new(&encoded) {
            if field == 2 {
                val = value.as_str();
            }
        }
        assert_eq!(val, "heat");
    }

    #[test]
    fn test_sensor_class_roundtrip() {
        let classes = [
            SensorClass::None,
            SensorClass::Temperature,
            SensorClass::Humidity,
            SensorClass::Battery,
            SensorClass::Pressure,
            SensorClass::Power,
            SensorClass::Energy,
            SensorClass::Voltage,
            SensorClass::Current,
            SensorClass::SignalStrength,
            SensorClass::Illuminance,
        ];

        for class in &classes {
            let s = class.as_str();
            if *class == SensorClass::None {
                assert_eq!(s, "");
            } else {
                assert_eq!(SensorClass::from_str(s), *class);
            }
        }
    }

    #[test]
    fn test_sensor_class_unknown() {
        assert_eq!(SensorClass::from_str("unknown"), SensorClass::None);
    }

    #[test]
    fn test_entity_state_missing() {
        let state = EntityState::Sensor { key: 1, value: 0.0, missing: true };
        let encoded = state.encode();

        let mut missing = false;
        for (field, value) in FieldIter::new(&encoded) {
            if field == 3 {
                missing = value.as_bool();
            }
        }
        assert!(missing);
    }
}
