use std::sync::{Arc, Mutex};

pub const NUM_THERMOSTATS: usize = 4;
pub const NUM_SENSORS: usize = 2;

pub struct Thermostat {
    pub name: &'static str,
    pub entity_id: &'static str,
    pub mode: String,
    pub hvac_action: String,
    pub current_temp: String,
    pub target_temp: String,
    pub target_low: String,
    pub target_high: String,
}

impl Thermostat {
    pub fn setpoint(&self) -> &str {
        if !self.target_temp.is_empty() {
            &self.target_temp
        } else if !self.target_low.is_empty() {
            &self.target_low
        } else {
            ""
        }
    }

    pub fn is_heating_mode(&self) -> bool {
        self.mode == "heat" || self.hvac_action == "heating"
    }

    pub fn is_cooling_mode(&self) -> bool {
        self.mode == "cool" || self.hvac_action == "cooling"
    }
}

impl Thermostat {
    pub fn display_status(&self) -> &str {
        if !self.hvac_action.is_empty() {
            &self.hvac_action
        } else if !self.mode.is_empty() {
            &self.mode
        } else {
            ""
        }
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
        if mins >= 330 && mins < 390 {
            (&self.early_morning_heat, &self.early_morning_cool)
        } else if mins >= 390 && mins < 510 {
            (&self.morning_heat, &self.morning_cool)
        } else if mins >= 510 && mins < 1260 {
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
    generation: u32,
}

impl DisplayState {
    pub fn new() -> Self {
        Self {
            thermostats: [
                Thermostat {
                    name: "Kitchen",
                    entity_id: "climate.kitchen_ac",
                    mode: String::new(),
                    hvac_action: String::new(),
                    current_temp: String::new(),
                    target_temp: String::new(),
                    target_low: String::new(),
                    target_high: String::new(),
                },
                Thermostat {
                    name: "Livingroom",
                    entity_id: "climate.livingroom_ac",
                    mode: String::new(),
                    hvac_action: String::new(),
                    current_temp: String::new(),
                    target_temp: String::new(),
                    target_low: String::new(),
                    target_high: String::new(),
                },
                Thermostat {
                    name: "Amos BR",
                    entity_id: "climate.amos_bedroom_ac",
                    mode: String::new(),
                    hvac_action: String::new(),
                    current_temp: String::new(),
                    target_temp: String::new(),
                    target_low: String::new(),
                    target_high: String::new(),
                },
                Thermostat {
                    name: "Hallway",
                    entity_id: "climate.smart_thermostat",
                    mode: String::new(),
                    hvac_action: String::new(),
                    current_temp: String::new(),
                    target_temp: String::new(),
                    target_low: String::new(),
                    target_high: String::new(),
                },
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
        }
        for s in &self.sensors {
            for eid in s.entity_ids {
                subs.push((eid.to_string(), String::new()));
            }
        }
        for eid in &SCHEDULE_ENTITIES {
            subs.push((eid.to_string(), String::new()));
        }
        subs
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
