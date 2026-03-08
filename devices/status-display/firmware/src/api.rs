use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use anyhow::Result;

use crate::proto::*;
use crate::slots::{MetricKind, SharedSlots, MAX_SLOTS};

const API_VERSION_MAJOR: u32 = 1;
const API_VERSION_MINOR: u32 = 10;
const API_VERSION_CURRENT: u32 = 10;
const DEVICE_NAME: &str = "status-display";
const FRIENDLY_NAME: &str = "Status Display";
const ESPHOME_VERSION: &str = "2024.1.0";
const MODEL: &str = "ESP32-2432S028";
const MANUFACTURER: &str = "Espressif";

const MSG_HELLO_REQ: u32 = 1;
const MSG_HELLO_RESP: u32 = 2;
const MSG_CONNECT_REQ: u32 = 3;
const MSG_CONNECT_RESP: u32 = 4;
const MSG_DISCONNECT_REQ: u32 = 5;
const MSG_DISCONNECT_RESP: u32 = 6;
const MSG_PING_REQ: u32 = 7;
const MSG_PING_RESP: u32 = 8;
const MSG_DEVICE_INFO_REQ: u32 = 9;
const MSG_DEVICE_INFO_RESP: u32 = 10;
const MSG_LIST_ENTITIES_REQ: u32 = 11;
const MSG_LIST_ENTITIES_SENSOR_RESP: u32 = 16;
const MSG_LIST_ENTITIES_TEXT_SENSOR_RESP: u32 = 18;
const MSG_LIST_ENTITIES_DONE: u32 = 19;
const MSG_SUBSCRIBE_STATES_REQ: u32 = 20;
const MSG_SENSOR_STATE_RESP: u32 = 25;
const MSG_TEXT_SENSOR_STATE_RESP: u32 = 27;
const MSG_SUBSCRIBE_HA_STATES_REQ: u32 = 38;
const MSG_SUBSCRIBE_HA_STATE_RESP: u32 = 39;
const MSG_HA_STATE_RESP: u32 = 40;
const MSG_LIST_ENTITIES_SELECT_RESP: u32 = 52;
const MSG_SELECT_STATE_RESP: u32 = 53;
const MSG_SELECT_CMD_REQ: u32 = 54;
const MSG_GET_TIME_REQ: u32 = 36;
const MSG_GET_TIME_RESP: u32 = 37;
const MSG_SUBSCRIBE_LOGS_REQ: u32 = 28;
const MSG_LIST_ENTITIES_TEXT_RESP: u32 = 97;
const MSG_TEXT_STATE_RESP: u32 = 98;
const MSG_TEXT_CMD_REQ: u32 = 99;

const KEY_BASE_SLOT_ENTITY: u32 = 0x1000;
const KEY_BASE_SLOT_LABEL: u32 = 0x2000;
const KEY_BASE_SLOT_KIND: u32 = 0x3000;
const KEY_BASE_SLOT_UNIT: u32 = 0x4000;
const KEY_BASE_SLOT_ATTR: u32 = 0x5000;
const KEY_BASE_SENSOR: u32 = 0x6000;
const KEY_BASE_TEXT_SENSOR: u32 = 0x7000;

pub fn start_server(slots: SharedSlots, mac: String, port: u16) {
    let listener = match TcpListener::bind(("0.0.0.0", port)) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to bind API server on port {}: {:?}", port, e);
            return;
        }
    };
    log::info!("ESPHome API server listening on port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let slots = slots.clone();
                let mac = mac.clone();
                std::thread::Builder::new()
                    .name("api-client".into())
                    .stack_size(16384)
                    .spawn(move || {
                        if let Err(e) = handle_client(stream, slots, &mac) {
                            log::warn!("API client disconnected: {:?}", e);
                        }
                    })
                    .ok();
            }
            Err(e) => log::warn!("Accept error: {:?}", e),
        }
    }
}

fn handle_client(mut stream: TcpStream, slots: SharedSlots, mac: &str) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(90)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.set_nodelay(true)?;
    log::info!("API client connected: {:?}", stream.peer_addr());

    let mut reader = FrameReader::new();
    let mut read_buf = [0u8; 1024];
    let mut subscribed_states = false;
    let mut prev_generation = 0u32;

    loop {
        match stream.read(&mut read_buf) {
            Ok(0) => return Ok(()),
            Ok(n) => reader.push(&read_buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
        }

        while let Some((msg_type, payload)) = reader.next_frame() {
            log::info!("RX msg_type={} len={}", msg_type, payload.len());
            match msg_type {
                MSG_HELLO_REQ => {
                    let mut client_info = "";
                    let mut client_major = 0u32;
                    let mut client_minor = 0u32;
                    for (field, value) in FieldIter::new(&payload) {
                        match field {
                            1 => client_info = value.as_str(),
                            2 => client_major = value.as_u32(),
                            3 => client_minor = value.as_u32(),
                            _ => {}
                        }
                    }
                    log::info!("Hello from '{}' API {}.{}", client_info, client_major, client_minor);
                    let resp = build_hello_response();
                    send(&mut stream, MSG_HELLO_RESP, &resp)?;
                }
                MSG_CONNECT_REQ => {
                    log::info!("TX ConnectResp");
                    send(&mut stream, MSG_CONNECT_RESP, &[])?;
                }
                MSG_DEVICE_INFO_REQ => {
                    let resp = build_device_info(mac);
                    send(&mut stream, MSG_DEVICE_INFO_RESP, &resp)?;
                }
                MSG_LIST_ENTITIES_REQ => {
                    log::info!("TX entity list...");
                    send_entity_list(&mut stream, &slots)?;
                    send(&mut stream, MSG_LIST_ENTITIES_DONE, &[])?;
                    log::info!("TX entity list done");
                }
                MSG_SUBSCRIBE_STATES_REQ => {
                    subscribed_states = true;
                    send_all_states(&mut stream, &slots)?;
                    prev_generation = slots.lock().unwrap().generation();
                }
                MSG_SUBSCRIBE_HA_STATES_REQ => {
                    send_ha_subscriptions(&mut stream, &slots)?;
                }
                MSG_HA_STATE_RESP => {
                    handle_ha_state(&payload, &slots);
                    if subscribed_states {
                        let gen = slots.lock().unwrap().generation();
                        if gen != prev_generation {
                            send_all_states(&mut stream, &slots)?;
                            prev_generation = gen;
                        }
                    }
                }
                MSG_SELECT_CMD_REQ => {
                    handle_select_command(&payload, &slots);
                    if subscribed_states {
                        send_all_states(&mut stream, &slots)?;
                        prev_generation = slots.lock().unwrap().generation();
                    }
                    send_ha_subscriptions(&mut stream, &slots)?;
                }
                MSG_TEXT_CMD_REQ => {
                    handle_text_command(&payload, &slots);
                    if subscribed_states {
                        send_all_states(&mut stream, &slots)?;
                        prev_generation = slots.lock().unwrap().generation();
                    }
                    send_ha_subscriptions(&mut stream, &slots)?;
                }
                MSG_PING_REQ => {
                    send(&mut stream, MSG_PING_RESP, &[])?;
                }
                MSG_DISCONNECT_REQ => {
                    send(&mut stream, MSG_DISCONNECT_RESP, &[])?;
                    return Ok(());
                }
                MSG_GET_TIME_REQ => {
                    send(&mut stream, MSG_GET_TIME_RESP, &[])?;
                }
                MSG_SUBSCRIBE_LOGS_REQ => {}
                _ => {
                    log::warn!("Unhandled msg type: {}", msg_type);
                }
            }
        }
    }
}

fn send(stream: &mut TcpStream, msg_type: u32, payload: &[u8]) -> Result<()> {
    let frame = frame_plaintext(msg_type, payload);
    stream.write_all(&frame)?;
    Ok(())
}

fn build_hello_response() -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_varint(1, API_VERSION_MAJOR as u64, &mut buf);
    encode_field_varint(2, API_VERSION_MINOR as u64, &mut buf);
    encode_field_string(3, &format!("{} {}", FRIENDLY_NAME, ESPHOME_VERSION), &mut buf);
    encode_field_string(4, DEVICE_NAME, &mut buf);
    buf
}

fn build_device_info(mac: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_bool(1, false, &mut buf);
    encode_field_string(2, DEVICE_NAME, &mut buf);
    encode_field_string(3, mac, &mut buf);
    encode_field_string(4, ESPHOME_VERSION, &mut buf);
    encode_field_string(5, "", &mut buf);
    encode_field_string(6, MODEL, &mut buf);
    encode_field_bool(7, false, &mut buf);
    encode_field_string(8, "status-display", &mut buf);
    encode_field_string(9, "0.2.0", &mut buf);
    encode_field_varint(10, 0, &mut buf);
    encode_field_string(12, MANUFACTURER, &mut buf);
    encode_field_string(13, FRIENDLY_NAME, &mut buf);
    buf
}

fn send_entity_list(stream: &mut TcpStream, slots: &SharedSlots) -> Result<()> {
    let mgr = slots.lock().unwrap();

    // Test: send one simple sensor to verify HA creates the device
    send(stream, MSG_LIST_ENTITIES_SENSOR_RESP, &build_sensor_entity(
        0x0001,
        "test_sensor",
        "Test Sensor",
        "°C",
    ))?;
    log::info!("Sent 1 test sensor entity");

    for i in 0..MAX_SLOTS {
        let slot = mgr.slot(i);
        send(stream, MSG_LIST_ENTITIES_TEXT_RESP, &build_text_entity(
            KEY_BASE_SLOT_ENTITY + i as u32,
            &format!("slot_{}_entity", i + 1),
            &format!("Slot {} Entity ID", i + 1),
        ))?;
        send(stream, MSG_LIST_ENTITIES_TEXT_RESP, &build_text_entity(
            KEY_BASE_SLOT_LABEL + i as u32,
            &format!("slot_{}_label", i + 1),
            &format!("Slot {} Label", i + 1),
        ))?;
        send(stream, MSG_LIST_ENTITIES_SELECT_RESP, &build_select_entity(
            KEY_BASE_SLOT_KIND + i as u32,
            &format!("slot_{}_kind", i + 1),
            &format!("Slot {} Display Type", i + 1),
            &["numeric", "text", "status"],
        ))?;
        send(stream, MSG_LIST_ENTITIES_TEXT_RESP, &build_text_entity(
            KEY_BASE_SLOT_UNIT + i as u32,
            &format!("slot_{}_unit", i + 1),
            &format!("Slot {} Unit", i + 1),
        ))?;
        send(stream, MSG_LIST_ENTITIES_TEXT_RESP, &build_text_entity(
            KEY_BASE_SLOT_ATTR + i as u32,
            &format!("slot_{}_attribute", i + 1),
            &format!("Slot {} Attribute", i + 1),
        ))?;
    }
    log::info!("Entity list complete");
    Ok(())
}

fn build_text_entity(key: u32, object_id: &str, name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_string(1, object_id, &mut buf);
    encode_field_fixed32(2, key, &mut buf);
    encode_field_string(3, name, &mut buf);
    encode_field_string(5, "mdi:form-textbox", &mut buf);
    encode_field_varint(7, 0, &mut buf); // entity_category = NONE
    encode_field_varint(8, 0, &mut buf); // min_length
    encode_field_varint(9, 255, &mut buf); // max_length
    buf
}

fn build_select_entity(key: u32, object_id: &str, name: &str, options: &[&str]) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_string(1, object_id, &mut buf);
    encode_field_fixed32(2, key, &mut buf);
    encode_field_string(3, name, &mut buf);
    encode_field_string(5, "mdi:cog", &mut buf);
    for opt in options {
        encode_field_string(6, opt, &mut buf);
    }
    encode_field_varint(8, 0, &mut buf); // entity_category = NONE
    buf
}

fn build_sensor_entity(key: u32, object_id: &str, name: &str, unit: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_string(1, object_id, &mut buf);
    encode_field_fixed32(2, key, &mut buf);
    encode_field_string(3, name, &mut buf);
    encode_field_string(6, unit, &mut buf);
    encode_field_varint(7, 1, &mut buf);
    buf
}

fn build_text_sensor_entity(key: u32, object_id: &str, name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_string(1, object_id, &mut buf);
    encode_field_fixed32(2, key, &mut buf);
    encode_field_string(3, name, &mut buf);
    buf
}

fn send_all_states(stream: &mut TcpStream, slots: &SharedSlots) -> Result<()> {
    let mgr = slots.lock().unwrap();

    // Test sensor state
    send(stream, MSG_SENSOR_STATE_RESP, &build_sensor_state(0x0001, 22.5, false))?;

    for i in 0..MAX_SLOTS {
        let slot = mgr.slot(i);

        send(stream, MSG_TEXT_STATE_RESP, &build_text_state(
            KEY_BASE_SLOT_ENTITY + i as u32, &slot.entity_id,
        ))?;
        send(stream, MSG_TEXT_STATE_RESP, &build_text_state(
            KEY_BASE_SLOT_LABEL + i as u32, &slot.label,
        ))?;
        send(stream, MSG_SELECT_STATE_RESP, &build_select_state(
            KEY_BASE_SLOT_KIND + i as u32, slot.kind.as_str(),
        ))?;
        send(stream, MSG_TEXT_STATE_RESP, &build_text_state(
            KEY_BASE_SLOT_UNIT + i as u32, &slot.unit,
        ))?;
        send(stream, MSG_TEXT_STATE_RESP, &build_text_state(
            KEY_BASE_SLOT_ATTR + i as u32, &slot.attribute,
        ))?;

        if !slot.entity_id.is_empty() {
            if slot.kind == MetricKind::Numeric {
                let val: f32 = slot.value.parse().unwrap_or(0.0);
                send(stream, MSG_SENSOR_STATE_RESP, &build_sensor_state(
                    KEY_BASE_SENSOR + i as u32, val, slot.value.is_empty(),
                ))?;
            } else {
                send(stream, MSG_TEXT_SENSOR_STATE_RESP, &build_text_sensor_state(
                    KEY_BASE_TEXT_SENSOR + i as u32, &slot.value, slot.value.is_empty(),
                ))?;
            }
        }
    }
    Ok(())
}

fn build_select_state(key: u32, state: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_fixed32(1, key, &mut buf);
    encode_field_string(2, state, &mut buf);
    encode_field_bool(3, state.is_empty(), &mut buf);
    buf
}

fn build_text_state(key: u32, state: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_fixed32(1, key, &mut buf);
    encode_field_string(2, state, &mut buf);
    encode_field_bool(3, state.is_empty(), &mut buf);
    buf
}

fn build_sensor_state(key: u32, value: f32, missing: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_fixed32(1, key, &mut buf);
    encode_field_float(2, value, &mut buf);
    encode_field_bool(3, missing, &mut buf);
    buf
}

fn build_text_sensor_state(key: u32, state: &str, missing: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_fixed32(1, key, &mut buf);
    encode_field_string(2, state, &mut buf);
    encode_field_bool(3, missing, &mut buf);
    buf
}

fn send_ha_subscriptions(stream: &mut TcpStream, slots: &SharedSlots) -> Result<()> {
    let subs = slots.lock().unwrap().active_subscriptions();
    for (entity_id, attribute) in &subs {
        let mut buf = Vec::new();
        encode_field_string(1, entity_id, &mut buf);
        encode_field_string(2, attribute, &mut buf);
        send(stream, MSG_SUBSCRIBE_HA_STATE_RESP, &buf)?;
    }
    Ok(())
}

fn handle_ha_state(payload: &[u8], slots: &SharedSlots) {
    let mut entity_id = "";
    let mut state = "";
    let mut attribute = "";

    for (field, value) in FieldIter::new(payload) {
        match field {
            1 => entity_id = value.as_str(),
            2 => state = value.as_str(),
            3 => attribute = value.as_str(),
            _ => {}
        }
    }

    if !entity_id.is_empty() {
        log::debug!("HA state: {} {} = {}", entity_id, attribute, state);
        slots.lock().unwrap().update_state(entity_id, attribute, state);
    }
}

fn handle_select_command(payload: &[u8], slots: &SharedSlots) {
    let mut key: u32 = 0;
    let mut state = String::new();

    for (field, value) in FieldIter::new(payload) {
        match field {
            1 => key = value.as_u32(),
            2 => state = value.as_str().to_string(),
            _ => {}
        }
    }

    let mut mgr = slots.lock().unwrap();

    for i in 0..MAX_SLOTS {
        if key == KEY_BASE_SLOT_KIND + i as u32 {
            log::info!("Slot {} kind = {}", i + 1, state);
            mgr.set_kind(i, MetricKind::from_str(&state));
            return;
        }
    }
    log::warn!("Unknown select key: 0x{:x}", key);
}

fn handle_text_command(payload: &[u8], slots: &SharedSlots) {
    let mut key: u32 = 0;
    let mut state = String::new();

    for (field, value) in FieldIter::new(payload) {
        match field {
            1 => key = value.as_u32(),
            2 => state = value.as_str().to_string(),
            _ => {}
        }
    }

    let mut mgr = slots.lock().unwrap();

    for i in 0..MAX_SLOTS {
        let idx = i;
        if key == KEY_BASE_SLOT_ENTITY + idx as u32 {
            log::info!("Slot {} entity_id = {}", i + 1, state);
            mgr.set_entity_id(idx, &state);
            return;
        }
        if key == KEY_BASE_SLOT_LABEL + idx as u32 {
            log::info!("Slot {} label = {}", i + 1, state);
            mgr.set_label(idx, &state);
            return;
        }
        if key == KEY_BASE_SLOT_UNIT + idx as u32 {
            log::info!("Slot {} unit = {}", i + 1, state);
            mgr.set_unit(idx, &state);
            return;
        }
        if key == KEY_BASE_SLOT_ATTR + idx as u32 {
            log::info!("Slot {} attribute = {}", i + 1, state);
            mgr.set_attribute(idx, &state);
            return;
        }
    }
    log::warn!("Unknown text key: 0x{:x}", key);
}
