use std::sync::{Arc, Mutex};

pub const NUM_THERMOSTATS: usize = 4;
pub const NUM_SENSORS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BbStatus {
    Unknown,
    Away,
    Working,
    Playing,
}

impl BbStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "away" => Self::Away,
            "working" => Self::Working,
            "playing" => Self::Playing,
            _ => Self::Unknown,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Unknown => "---",
            Self::Away => "Away",
            Self::Working => "Working",
            Self::Playing => "Playing",
        }
    }
}

/// What a thermostat is actually doing right now, as shown on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Heating,
    Cooling,
    Drying,
    Defrosting,
    Fan,
    Idle,
    Off,
    /// No `hvac_action` reported; only the selected mode is known.
    Auto,
    Unknown,
}

impl Action {
    fn from_ha(hvac_action: &str, mode: &str) -> Self {
        match hvac_action {
            "heating" | "preheating" => return Self::Heating,
            "cooling" => return Self::Cooling,
            "drying" => return Self::Drying,
            "defrosting" => return Self::Defrosting,
            "fan" => return Self::Fan,
            "idle" => return Self::Idle,
            "off" => return Self::Off,
            _ => {}
        }
        match mode {
            "off" => Self::Off,
            "fan_only" => Self::Fan,
            "" | "unknown" | "unavailable" => Self::Unknown,
            _ => Self::Auto,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Heating => "HEATING",
            Self::Cooling => "COOLING",
            Self::Drying => "DRYING",
            Self::Defrosting => "DEFROST",
            Self::Fan => "FAN ONLY",
            Self::Idle => "IDLE",
            Self::Off => "OFF",
            Self::Auto => "AUTO",
            Self::Unknown => "---",
        }
    }
}

pub struct Thermostat {
    pub name: &'static str,
    pub entity_id: &'static str,
    /// binary_sensor reporting whether the indoor fan is actually spinning.
    pub fan_entity: Option<&'static str>,
    pub mode: String,
    pub hvac_action: String,
    pub current_temp: String,
    pub target_temp: String,
    pub target_low: String,
    pub target_high: String,
    pub fan_running: Option<bool>,
}

impl Thermostat {
    const fn new(name: &'static str, entity_id: &'static str, fan_entity: Option<&'static str>) -> Self {
        Self {
            name,
            entity_id,
            fan_entity,
            mode: String::new(),
            hvac_action: String::new(),
            current_temp: String::new(),
            target_temp: String::new(),
            target_low: String::new(),
            target_high: String::new(),
            fan_running: None,
        }
    }

    pub fn action(&self) -> Action {
        Action::from_ha(&self.hvac_action, &self.mode)
    }

    /// Indoor fan running: from the dedicated sensor when there is one,
    /// otherwise implied by a fan-only action.
    pub fn fan_on(&self) -> bool {
        self.fan_running.unwrap_or(self.action() == Action::Fan)
    }
}

pub struct ExtraSensor {
    pub name: &'static str,
    pub entity_ids: &'static [&'static str],
    pub value: String,
}

const SCHEDULE_ENTITIES: [&str; 8] = [
    "input_number.hvac_early_morning_heat_f",
    "input_number.hvac_early_morning_cool_f",
    "input_number.hvac_morning_heat_f",
    "input_number.hvac_morning_cool_f",
    "input_number.hvac_daytime_heat_f",
    "input_number.hvac_daytime_cool_f",
    "input_number.hvac_overnight_heat_f",
    "input_number.hvac_overnight_cool_f",
];

pub struct ScheduleBand {
    pub early_morning_heat: String,
    pub early_morning_cool: String,
    pub morning_heat: String,
    pub morning_cool: String,
    pub daytime_heat: String,
    pub daytime_cool: String,
    pub overnight_heat: String,
    pub overnight_cool: String,
}

impl ScheduleBand {
    fn new() -> Self {
        Self {
            early_morning_heat: String::new(),
            early_morning_cool: String::new(),
            morning_heat: String::new(),
            morning_cool: String::new(),
            daytime_heat: String::new(),
            daytime_cool: String::new(),
            overnight_heat: String::new(),
            overnight_cool: String::new(),
        }
    }

    pub fn current(&self, hour: u8, minute: u8) -> (&str, &str) {
        let mins = hour as u16 * 60 + minute as u16;
        if (330..390).contains(&mins) {
            (&self.early_morning_heat, &self.early_morning_cool)
        } else if (390..510).contains(&mins) {
            (&self.morning_heat, &self.morning_cool)
        } else if (510..1260).contains(&mins) {
            (&self.daytime_heat, &self.daytime_cool)
        } else {
            (&self.overnight_heat, &self.overnight_cool)
        }
    }

    fn update(&mut self, entity_id: &str, value: &str) -> bool {
        let field = match entity_id {
            "input_number.hvac_early_morning_heat_f" => &mut self.early_morning_heat,
            "input_number.hvac_early_morning_cool_f" => &mut self.early_morning_cool,
            "input_number.hvac_morning_heat_f" => &mut self.morning_heat,
            "input_number.hvac_morning_cool_f" => &mut self.morning_cool,
            "input_number.hvac_daytime_heat_f" => &mut self.daytime_heat,
            "input_number.hvac_daytime_cool_f" => &mut self.daytime_cool,
            "input_number.hvac_overnight_heat_f" => &mut self.overnight_heat,
            "input_number.hvac_overnight_cool_f" => &mut self.overnight_cool,
            _ => return false,
        };
        set_if_changed(field, value)
    }
}

pub struct DisplayState {
    pub thermostats: [Thermostat; NUM_THERMOSTATS],
    pub sensors: [ExtraSensor; NUM_SENSORS],
    pub schedule: ScheduleBand,
    pub bb_status: BbStatus,
    /// Number of Home Assistant API connections currently open.
    pub ha_clients: u32,
    generation: u32,
}

impl DisplayState {
    pub fn new() -> Self {
        Self {
            thermostats: [
                Thermostat::new(
                    "Kitchen",
                    "climate.minisplit_kitchen_smart_thermostat",
                    Some("binary_sensor.kitchen_minisplit_kitchen_fan"),
                ),
                Thermostat::new(
                    "Living Room",
                    "climate.minisplit_livingroom_smart_thermostat",
                    Some("binary_sensor.living_room_minisplit_livingroom_fan"),
                ),
                Thermostat::new(
                    "Amos BR",
                    "climate.minisplit_amos_smart_thermostat",
                    Some("binary_sensor.amos_bedroom_minisplit_amos_fan"),
                ),
                Thermostat::new("Hallway", "climate.smart_thermostat", None),
            ],
            sensors: [
                ExtraSensor {
                    name: "A&K BR",
                    entity_ids: &["sensor.atc_a0c6_temperature"],
                    value: String::new(),
                },
                ExtraSensor {
                    name: "Kitchen",
                    entity_ids: &["sensor.kitchen_temperature", "sensor.kitchen_temp_temperature"],
                    value: String::new(),
                },
            ],
            schedule: ScheduleBand::new(),
            bb_status: BbStatus::Unknown,
            ha_clients: 0,
            generation: 0,
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn subscriptions(&self) -> Vec<(String, String)> {
        let mut subs = Vec::new();
        for t in &self.thermostats {
            subs.push((t.entity_id.to_string(), String::new()));
            subs.push((t.entity_id.to_string(), "hvac_action".to_string()));
            subs.push((t.entity_id.to_string(), "current_temperature".to_string()));
            subs.push((t.entity_id.to_string(), "temperature".to_string()));
            subs.push((t.entity_id.to_string(), "target_temp_low".to_string()));
            subs.push((t.entity_id.to_string(), "target_temp_high".to_string()));
            if let Some(fan) = t.fan_entity {
                subs.push((fan.to_string(), String::new()));
            }
        }
        for s in &self.sensors {
            for eid in s.entity_ids {
                subs.push((eid.to_string(), String::new()));
            }
        }
        for eid in &SCHEDULE_ENTITIES {
            subs.push((eid.to_string(), String::new()));
        }
        subs.push(("input_text.bb_status".to_string(), String::new()));
        subs
    }

    pub fn set_ha_clients(&mut self, clients: u32) {
        if self.ha_clients != clients {
            self.ha_clients = clients;
            self.generation += 1;
        }
    }

    pub fn update(&mut self, entity_id: &str, attribute: &str, value: &str) {
        for t in &mut self.thermostats {
            if t.entity_id == entity_id {
                let changed = match attribute {
                    "" => set_if_changed(&mut t.mode, value),
                    "hvac_action" => set_if_changed(&mut t.hvac_action, value),
                    "current_temperature" => set_if_changed(&mut t.current_temp, value),
                    "temperature" => set_if_changed(&mut t.target_temp, value),
                    "target_temp_low" => set_if_changed(&mut t.target_low, value),
                    "target_temp_high" => set_if_changed(&mut t.target_high, value),
                    _ => false,
                };
                if changed {
                    self.generation += 1;
                }
                return;
            }
            if attribute.is_empty() && t.fan_entity == Some(entity_id) {
                let running = match value {
                    "on" => Some(true),
                    "off" => Some(false),
                    _ => None,
                };
                if t.fan_running != running {
                    t.fan_running = running;
                    self.generation += 1;
                }
                return;
            }
        }
        if attribute.is_empty() {
            if self.schedule.update(entity_id, value) {
                self.generation += 1;
                return;
            }
            for s in &mut self.sensors {
                if s.entity_ids.contains(&entity_id) {
                    if set_if_changed(&mut s.value, value) {
                        self.generation += 1;
                    }
                    return;
                }
            }
            if entity_id == "input_text.bb_status" {
                let new_status = BbStatus::from_str(value);
                if new_status != self.bb_status {
                    self.bb_status = new_status;
                    self.generation += 1;
                }
            }
        }
    }
}

fn set_if_changed(field: &mut String, value: &str) -> bool {
    if field.as_str() != value {
        field.clear();
        field.push_str(value);
        true
    } else {
        false
    }
}

pub type SharedState = Arc<Mutex<DisplayState>>;

pub fn new_shared() -> SharedState {
    Arc::new(Mutex::new(DisplayState::new()))
}
