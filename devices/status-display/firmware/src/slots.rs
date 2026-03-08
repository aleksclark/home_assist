use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum MetricKind {
    Numeric,
    Text,
    Status,
}

impl MetricKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "text" => Self::Text,
            "status" => Self::Status,
            _ => Self::Numeric,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::Text => "text",
            Self::Status => "status",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusMapping {
    pub value: String,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub label: String,
    pub entity_id: String,
    pub attribute: String,
    pub kind: MetricKind,
    pub unit: String,
    pub value: String,
    pub status_map: Vec<StatusMapping>,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            label: String::new(),
            entity_id: String::new(),
            attribute: String::new(),
            kind: MetricKind::Text,
            unit: String::new(),
            value: String::new(),
            status_map: Vec::new(),
        }
    }
}

pub const MAX_SLOTS: usize = 6;

pub struct SlotManager {
    slots: [Slot; MAX_SLOTS],
    generation: u32,
}

impl SlotManager {
    pub fn new() -> Self {
        let slots = core::array::from_fn(|i| Slot {
            label: format!("Slot {}", i + 1),
            ..Default::default()
        });
        Self { slots, generation: 0 }
    }

    pub fn slot(&self, idx: usize) -> &Slot {
        &self.slots[idx]
    }

    pub fn slot_mut(&mut self, idx: usize) -> &mut Slot {
        &mut self.slots[idx]
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn set_entity_id(&mut self, idx: usize, entity_id: &str) {
        if idx < MAX_SLOTS && self.slots[idx].entity_id != entity_id {
            self.slots[idx].entity_id = entity_id.to_string();
            self.slots[idx].value.clear();
            self.generation += 1;
        }
    }

    pub fn set_label(&mut self, idx: usize, label: &str) {
        if idx < MAX_SLOTS && self.slots[idx].label != label {
            self.slots[idx].label = label.to_string();
            self.generation += 1;
        }
    }

    pub fn set_kind(&mut self, idx: usize, kind: MetricKind) {
        if idx < MAX_SLOTS && self.slots[idx].kind != kind {
            self.slots[idx].kind = kind;
            self.generation += 1;
        }
    }

    pub fn set_unit(&mut self, idx: usize, unit: &str) {
        if idx < MAX_SLOTS && self.slots[idx].unit != unit {
            self.slots[idx].unit = unit.to_string();
            self.generation += 1;
        }
    }

    pub fn set_attribute(&mut self, idx: usize, attr: &str) {
        if idx < MAX_SLOTS && self.slots[idx].attribute != attr {
            self.slots[idx].attribute = attr.to_string();
            self.generation += 1;
        }
    }

    pub fn set_value(&mut self, idx: usize, value: &str) {
        if idx < MAX_SLOTS {
            self.slots[idx].value = value.to_string();
            self.generation += 1;
        }
    }

    pub fn update_state(&mut self, entity_id: &str, attribute: &str, state: &str) {
        for slot in &mut self.slots {
            if slot.entity_id == entity_id
                && ((attribute.is_empty() && slot.attribute.is_empty())
                    || slot.attribute == attribute)
            {
                if slot.value != state {
                    slot.value = state.to_string();
                    self.generation += 1;
                }
            }
        }
    }

    pub fn active_subscriptions(&self) -> Vec<(String, String)> {
        let mut subs = Vec::new();
        for slot in &self.slots {
            if !slot.entity_id.is_empty() {
                let pair = (slot.entity_id.clone(), slot.attribute.clone());
                if !subs.contains(&pair) {
                    subs.push(pair);
                }
            }
        }
        subs
    }

    pub fn set_status_map(&mut self, idx: usize, map: Vec<StatusMapping>) {
        if idx < MAX_SLOTS {
            self.slots[idx].status_map = map;
            self.generation += 1;
        }
    }
}

pub type SharedSlots = Arc<Mutex<SlotManager>>;

pub fn new_shared() -> SharedSlots {
    Arc::new(Mutex::new(SlotManager::new()))
}
