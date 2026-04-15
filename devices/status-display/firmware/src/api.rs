use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use anyhow::Result;

use crate::proto::*;
use crate::state::SharedState;

const API_VERSION_MAJOR: u32 = 1;
const API_VERSION_MINOR: u32 = 10;
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
const MSG_LIST_ENTITIES_DONE: u32 = 19;
const MSG_SUBSCRIBE_STATES_REQ: u32 = 20;
const MSG_GET_HA_STATES_REQ: u32 = 38;
const MSG_SUBSCRIBE_HA_STATE_RESP: u32 = 39;
const MSG_HA_STATE_RESP: u32 = 40;
const MSG_GET_TIME_REQ: u32 = 36;
const MSG_GET_TIME_RESP: u32 = 37;
const MSG_SUBSCRIBE_LOGS_REQ: u32 = 28;

use std::sync::Mutex;

static OTA_STREAM: Mutex<Option<TcpStream>> = Mutex::new(None);

pub fn take_ota_stream() -> Option<TcpStream> {
    OTA_STREAM.lock().ok()?.take()
}

pub fn start_server(state: SharedState, mac: String, port: u16) {
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
            Ok(mut stream) => {
                stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
                let mut magic = [0u8; 2];
                match stream.read_exact(&mut magic) {
                    Ok(()) if &magic == b"OT" => {
                        log::info!("OTA connection received");
                        if let Ok(mut slot) = OTA_STREAM.lock() {
                            *slot = Some(stream);
                        }
                        continue;
                    }
                    Ok(()) => {}
                    Err(e) => {
                        log::warn!("Read magic failed: {:?}", e);
                        continue;
                    }
                }
                let state = state.clone();
                let mac = mac.clone();
                if let Err(e) = std::thread::Builder::new()
                    .name("api-client".into())
                    .stack_size(32768)
                    .spawn(move || {
                        if let Err(e) = handle_client_with_prefix(stream, state, &mac, &magic) {
                            log::warn!("API client disconnected: {:?}", e);
                        }
                    })
                {
                    log::error!("Failed to spawn client thread: {:?}", e);
                }
            }
            Err(e) => log::warn!("Accept error: {:?}", e),
        }
    }
}

fn handle_client_with_prefix(stream: TcpStream, state: SharedState, mac: &str, prefix: &[u8; 2]) -> Result<()> {
    let mut reader = FrameReader::new();
    reader.push(prefix);
    handle_client_inner(stream, state, mac, reader)
}

fn handle_client_inner(mut stream: TcpStream, state: SharedState, mac: &str, mut reader: FrameReader) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(90)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.set_nodelay(true)?;
    log::info!("API client connected: {:?}", stream.peer_addr());

    let mut read_buf = [0u8; 512];

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
                    let resp = build_hello_response();
                    send(&mut stream, MSG_HELLO_RESP, &resp)?;
                }
                MSG_CONNECT_REQ => {
                    send(&mut stream, MSG_CONNECT_RESP, &[])?;
                }
                MSG_DEVICE_INFO_REQ => {
                    let resp = build_device_info(mac);
                    send(&mut stream, MSG_DEVICE_INFO_RESP, &resp)?;
                }
                MSG_LIST_ENTITIES_REQ => {
                    send(&mut stream, MSG_LIST_ENTITIES_DONE, &[])?;
                }
                MSG_SUBSCRIBE_STATES_REQ => {}
                MSG_GET_HA_STATES_REQ => {
                    send_ha_subscriptions(&mut stream, &state)?;
                }
                MSG_HA_STATE_RESP => {
                    handle_ha_state(&payload, &state);
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
    encode_field_string(2, DEVICE_NAME, &mut buf);
    encode_field_string(3, mac, &mut buf);
    encode_field_string(4, ESPHOME_VERSION, &mut buf);
    encode_field_string(6, MODEL, &mut buf);
    encode_field_string(8, "resphome.status-display", &mut buf);
    encode_field_string(9, "0.3.0", &mut buf);
    encode_field_string(12, MANUFACTURER, &mut buf);
    encode_field_string(13, FRIENDLY_NAME, &mut buf);
    buf
}

fn send_ha_subscriptions(stream: &mut TcpStream, state: &SharedState) -> Result<()> {
    let subs = state.lock().unwrap().subscriptions();
    for (entity_id, attribute) in &subs {
        let mut buf = Vec::new();
        encode_field_string(1, entity_id, &mut buf);
        if !attribute.is_empty() {
            encode_field_string(2, attribute, &mut buf);
        }
        send(stream, MSG_SUBSCRIBE_HA_STATE_RESP, &buf)?;
    }
    Ok(())
}

fn handle_ha_state(payload: &[u8], state: &SharedState) {
    let mut entity_id = "";
    let mut value = "";
    let mut attribute = "";

    for (field, fv) in FieldIter::new(payload) {
        match field {
            1 => entity_id = fv.as_str(),
            2 => value = fv.as_str(),
            3 => attribute = fv.as_str(),
            _ => {}
        }
    }

    if !entity_id.is_empty() {
        log::info!("HA state: {} [{}] = {}", entity_id, attribute, value);
        state.lock().unwrap().update(entity_id, attribute, value);
    }
}
