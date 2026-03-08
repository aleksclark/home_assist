use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::{debug, info, warn};

use crate::device::DeviceInfo;
use crate::entity::{Entity, EntityState};
use crate::proto::*;

pub const ESPHOME_VERSION: &str = "2024.1.0";

pub mod msg {
    pub const HELLO_REQ: u32 = 1;
    pub const HELLO_RESP: u32 = 2;
    pub const CONNECT_REQ: u32 = 3;
    pub const CONNECT_RESP: u32 = 4;
    pub const DISCONNECT_REQ: u32 = 5;
    pub const DISCONNECT_RESP: u32 = 6;
    pub const PING_REQ: u32 = 7;
    pub const PING_RESP: u32 = 8;
    pub const DEVICE_INFO_REQ: u32 = 9;
    pub const DEVICE_INFO_RESP: u32 = 10;
    pub const LIST_ENTITIES_REQ: u32 = 11;
    pub const LIST_ENTITIES_BINARY_SENSOR_RESP: u32 = 12;
    pub const LIST_ENTITIES_SENSOR_RESP: u32 = 16;
    pub const LIST_ENTITIES_TEXT_SENSOR_RESP: u32 = 18;
    pub const LIST_ENTITIES_DONE: u32 = 19;
    pub const SUBSCRIBE_STATES_REQ: u32 = 20;
    pub const BINARY_SENSOR_STATE_RESP: u32 = 21;
    pub const SENSOR_STATE_RESP: u32 = 25;
    pub const TEXT_SENSOR_STATE_RESP: u32 = 27;
    pub const SUBSCRIBE_LOGS_REQ: u32 = 28;
    pub const GET_TIME_REQ: u32 = 36;
    pub const GET_TIME_RESP: u32 = 37;
    pub const SUBSCRIBE_HA_STATES_REQ: u32 = 38;
    pub const SUBSCRIBE_HA_STATE_RESP: u32 = 39;
    pub const HA_STATE_RESP: u32 = 40;
    pub const LIST_ENTITIES_SELECT_RESP: u32 = 52;
    pub const SELECT_STATE_RESP: u32 = 53;
    pub const SELECT_CMD_REQ: u32 = 54;
    pub const LIST_ENTITIES_BLE_RESP: u32 = 58;
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub port: u16,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub password: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: 6053,
            read_timeout: Duration::from_secs(90),
            write_timeout: Duration::from_secs(10),
            password: None,
        }
    }
}

impl ApiConfig {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            ..Default::default()
        }
    }
}

pub trait ClientHandler: Send + Sync {
    fn device_info(&self) -> DeviceInfo;
    fn list_entities(&self) -> Vec<Entity>;
    fn get_states(&self) -> Vec<EntityState>;
    fn on_ha_state(&self, entity_id: &str, attribute: &str, state: &str);
    fn on_select_command(&self, key: u32, value: &str);
    fn ha_subscriptions(&self) -> Vec<(String, String)>;
}

pub struct ApiServer {
    config: ApiConfig,
    handler: Arc<dyn ClientHandler>,
}

impl ApiServer {
    pub fn new(config: ApiConfig, handler: Arc<dyn ClientHandler>) -> Self {
        Self { config, handler }
    }

    pub fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", self.config.port))?;
        info!("ESPHome API server listening on port {}", self.config.port);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handler = self.handler.clone();
                    let config = self.config.clone();
                    std::thread::Builder::new()
                        .name("api-client".into())
                        .stack_size(16384)
                        .spawn(move || {
                            if let Err(e) = handle_client(stream, &config, handler.as_ref()) {
                                warn!("API client disconnected: {:?}", e);
                            }
                        })
                        .ok();
                }
                Err(e) => warn!("Accept error: {:?}", e),
            }
        }
        Ok(())
    }
}

fn send(stream: &mut TcpStream, msg_type: u32, payload: &[u8]) -> Result<()> {
    let frame = frame_plaintext(msg_type, payload);
    stream.write_all(&frame)?;
    Ok(())
}

fn build_hello_response(device_info: &DeviceInfo) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_varint(1, 1, &mut buf); // api_version_major
    encode_field_varint(2, 10, &mut buf); // api_version_minor
    let server_info = format!("{} {}", device_info.friendly_name, device_info.esphome_version);
    encode_field_string(3, &server_info, &mut buf);
    encode_field_string(4, &device_info.name, &mut buf);
    buf
}

fn send_entity_list(
    stream: &mut TcpStream,
    handler: &dyn ClientHandler,
) -> Result<()> {
    let entities = handler.list_entities();
    for entity in &entities {
        match entity {
            Entity::Sensor(e) => {
                send(stream, msg::LIST_ENTITIES_SENSOR_RESP, &e.encode_list_entry())?;
            }
            Entity::TextSensor(e) => {
                send(stream, msg::LIST_ENTITIES_TEXT_SENSOR_RESP, &e.encode_list_entry())?;
            }
            Entity::BinarySensor(e) => {
                send(stream, msg::LIST_ENTITIES_BINARY_SENSOR_RESP, &e.encode_list_entry())?;
            }
            Entity::Select(e) => {
                send(stream, msg::LIST_ENTITIES_SELECT_RESP, &e.encode_list_entry())?;
            }
        }
    }
    send(stream, msg::LIST_ENTITIES_DONE, &[])?;
    info!("Sent {} entities", entities.len());
    Ok(())
}

fn send_all_states(
    stream: &mut TcpStream,
    handler: &dyn ClientHandler,
) -> Result<()> {
    let states = handler.get_states();
    for state in &states {
        send(stream, state.msg_type(), &state.encode())?;
    }
    debug!("Sent {} state updates", states.len());
    Ok(())
}

fn send_ha_subscriptions(
    stream: &mut TcpStream,
    handler: &dyn ClientHandler,
) -> Result<()> {
    let subs = handler.ha_subscriptions();
    for (entity_id, attribute) in &subs {
        let mut buf = Vec::new();
        encode_field_string(1, entity_id, &mut buf);
        encode_field_string(2, attribute, &mut buf);
        send(stream, msg::SUBSCRIBE_HA_STATE_RESP, &buf)?;
    }
    Ok(())
}

fn handle_ha_state(payload: &[u8], handler: &dyn ClientHandler) {
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
        debug!("HA state: {} {} = {}", entity_id, attribute, state);
        handler.on_ha_state(entity_id, attribute, state);
    }
}

fn handle_select_command(payload: &[u8], handler: &dyn ClientHandler) {
    let mut key: u32 = 0;
    let mut state = "";

    for (field, value) in FieldIter::new(payload) {
        match field {
            1 => key = value.as_u32(),
            2 => state = value.as_str(),
            _ => {}
        }
    }

    info!("Select command: key=0x{:x} state={}", key, state);
    handler.on_select_command(key, state);
}

fn handle_client(
    mut stream: TcpStream,
    config: &ApiConfig,
    handler: &dyn ClientHandler,
) -> Result<()> {
    stream.set_read_timeout(Some(config.read_timeout))?;
    stream.set_write_timeout(Some(config.write_timeout))?;
    stream.set_nodelay(true)?;
    info!("API client connected: {:?}", stream.peer_addr());

    let mut reader = FrameReader::new();
    let mut read_buf = [0u8; 1024];
    let mut subscribed_states = false;

    loop {
        match stream.read(&mut read_buf) {
            Ok(0) => return Ok(()),
            Ok(n) => reader.push(&read_buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
        }

        while let Some((msg_type, payload)) = reader.next_frame() {
            debug!("RX msg_type={} len={}", msg_type, payload.len());
            match msg_type {
                msg::HELLO_REQ => {
                    let device_info = handler.device_info();
                    let resp = build_hello_response(&device_info);
                    send(&mut stream, msg::HELLO_RESP, &resp)?;
                }
                msg::CONNECT_REQ => {
                    send(&mut stream, msg::CONNECT_RESP, &[])?;
                }
                msg::DEVICE_INFO_REQ => {
                    let info = handler.device_info();
                    send(&mut stream, msg::DEVICE_INFO_RESP, &info.encode())?;
                }
                msg::LIST_ENTITIES_REQ => {
                    send_entity_list(&mut stream, handler)?;
                }
                msg::SUBSCRIBE_STATES_REQ => {
                    subscribed_states = true;
                    send_all_states(&mut stream, handler)?;
                }
                msg::SUBSCRIBE_HA_STATES_REQ => {
                    send_ha_subscriptions(&mut stream, handler)?;
                }
                msg::HA_STATE_RESP => {
                    handle_ha_state(&payload, handler);
                    if subscribed_states {
                        send_all_states(&mut stream, handler)?;
                    }
                }
                msg::SELECT_CMD_REQ => {
                    handle_select_command(&payload, handler);
                    if subscribed_states {
                        send_all_states(&mut stream, handler)?;
                    }
                    send_ha_subscriptions(&mut stream, handler)?;
                }
                msg::PING_REQ => {
                    send(&mut stream, msg::PING_RESP, &[])?;
                }
                msg::DISCONNECT_REQ => {
                    send(&mut stream, msg::DISCONNECT_RESP, &[])?;
                    return Ok(());
                }
                msg::GET_TIME_REQ => {
                    send(&mut stream, msg::GET_TIME_RESP, &[])?;
                }
                msg::SUBSCRIBE_LOGS_REQ => {}
                _ => {
                    warn!("Unhandled msg type: {}", msg_type);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use crate::device::DeviceConfig;

    struct MockHandler {
        device: DeviceInfo,
        entities: Vec<Entity>,
        states: Vec<EntityState>,
        ha_states: Arc<Mutex<Vec<(String, String, String)>>>,
        select_cmds: Arc<Mutex<Vec<(u32, String)>>>,
    }

    impl MockHandler {
        fn new() -> Self {
            let config = DeviceConfig::new("test", "Test Device")
                .with_mac("AA:BB:CC:DD:EE:FF")
                .with_model("ESP32-WROOM-32");
            Self {
                device: DeviceInfo::from_config(&config, ESPHOME_VERSION),
                entities: vec![
                    Entity::Sensor(crate::entity::SensorEntity::new(1, "temp", "Temperature").with_unit("°C")),
                    Entity::BinarySensor(crate::entity::BinarySensorEntity::new(2, "status", "Status").as_status_sensor()),
                ],
                states: vec![
                    EntityState::Sensor { key: 1, value: 22.5, missing: false },
                    EntityState::BinarySensor { key: 2, state: true, missing: false },
                ],
                ha_states: Arc::new(Mutex::new(Vec::new())),
                select_cmds: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl ClientHandler for MockHandler {
        fn device_info(&self) -> DeviceInfo {
            self.device.clone()
        }

        fn list_entities(&self) -> Vec<Entity> {
            self.entities.clone()
        }

        fn get_states(&self) -> Vec<EntityState> {
            self.states.clone()
        }

        fn on_ha_state(&self, entity_id: &str, attribute: &str, state: &str) {
            self.ha_states.lock().unwrap().push((
                entity_id.to_string(),
                attribute.to_string(),
                state.to_string(),
            ));
        }

        fn on_select_command(&self, key: u32, value: &str) {
            self.select_cmds.lock().unwrap().push((key, value.to_string()));
        }

        fn ha_subscriptions(&self) -> Vec<(String, String)> {
            vec![("sensor.temperature".to_string(), "".to_string())]
        }
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.port, 6053);
        assert_eq!(config.read_timeout, Duration::from_secs(90));
        assert_eq!(config.write_timeout, Duration::from_secs(10));
        assert!(config.password.is_none());
    }

    #[test]
    fn test_api_config_custom_port() {
        let config = ApiConfig::new(8080);
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_build_hello_response() {
        let config = DeviceConfig::new("test-dev", "Test Device");
        let info = DeviceInfo::from_config(&config, "2024.1.0");
        let resp = build_hello_response(&info);

        let mut major = 0u32;
        let mut minor = 0u32;
        let mut server_info = "";
        let mut name = "";

        for (field, value) in FieldIter::new(&resp) {
            match field {
                1 => major = value.as_u32(),
                2 => minor = value.as_u32(),
                3 => server_info = value.as_str(),
                4 => name = value.as_str(),
                _ => {}
            }
        }

        assert_eq!(major, 1);
        assert_eq!(minor, 10);
        assert!(server_info.contains("Test Device"));
        assert!(server_info.contains("2024.1.0"));
        assert_eq!(name, "test-dev");
    }

    #[test]
    fn test_mock_handler_entities() {
        let handler = MockHandler::new();
        let entities = handler.list_entities();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].key(), 1);
        assert_eq!(entities[1].key(), 2);
    }

    #[test]
    fn test_mock_handler_states() {
        let handler = MockHandler::new();
        let states = handler.get_states();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].key(), 1);
        assert_eq!(states[1].key(), 2);
    }

    #[test]
    fn test_mock_handler_ha_state() {
        let handler = MockHandler::new();
        handler.on_ha_state("sensor.temp", "", "22.5");
        let states = handler.ha_states.lock().unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].0, "sensor.temp");
        assert_eq!(states[0].2, "22.5");
    }

    #[test]
    fn test_mock_handler_select_command() {
        let handler = MockHandler::new();
        handler.on_select_command(0x100, "heat");
        let cmds = handler.select_cmds.lock().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], (0x100, "heat".to_string()));
    }

    #[test]
    fn test_handle_ha_state_parse() {
        let handler = MockHandler::new();

        let mut payload = Vec::new();
        encode_field_string(1, "climate.living_room", &mut payload);
        encode_field_string(2, "heat", &mut payload);
        encode_field_string(3, "hvac_action", &mut payload);

        handle_ha_state(&payload, &handler);

        let states = handler.ha_states.lock().unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].0, "climate.living_room");
        assert_eq!(states[0].1, "hvac_action");
        assert_eq!(states[0].2, "heat");
    }

    #[test]
    fn test_handle_select_command_parse() {
        let handler = MockHandler::new();

        let mut payload = Vec::new();
        encode_field_varint(1, 0x3000, &mut payload);
        encode_field_string(2, "cool", &mut payload);

        handle_select_command(&payload, &handler);

        let cmds = handler.select_cmds.lock().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].0, 0x3000);
        assert_eq!(cmds[0].1, "cool");
    }

    #[test]
    fn test_handle_ha_state_empty_entity() {
        let handler = MockHandler::new();

        let payload = Vec::new(); // empty
        handle_ha_state(&payload, &handler);

        let states = handler.ha_states.lock().unwrap();
        assert!(states.is_empty());
    }

    #[test]
    fn test_msg_constants() {
        assert_eq!(msg::HELLO_REQ, 1);
        assert_eq!(msg::HELLO_RESP, 2);
        assert_eq!(msg::PING_REQ, 7);
        assert_eq!(msg::PING_RESP, 8);
        assert_eq!(msg::DEVICE_INFO_REQ, 9);
        assert_eq!(msg::LIST_ENTITIES_DONE, 19);
        assert_eq!(msg::SENSOR_STATE_RESP, 25);
        assert_eq!(msg::TEXT_SENSOR_STATE_RESP, 27);
    }

    #[test]
    fn test_full_protocol_exchange() {
        use std::net::TcpListener;
        use std::thread;

        let handler = Arc::new(MockHandler::new());

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handler = handler.clone();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let config = ApiConfig::default();
            handle_client(stream, &config, server_handler.as_ref()).ok();
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

        // Send HelloRequest
        let mut hello_payload = Vec::new();
        encode_field_string(1, "test-client", &mut hello_payload);
        encode_field_varint(2, 1, &mut hello_payload);
        encode_field_varint(3, 10, &mut hello_payload);
        let frame = frame_plaintext(msg::HELLO_REQ, &hello_payload);
        client.write_all(&frame).unwrap();

        // Read HelloResponse
        let mut buf = [0u8; 256];
        let n = client.read(&mut buf).unwrap();
        assert!(n > 0);
        let mut reader = FrameReader::new();
        reader.push(&buf[..n]);
        let (resp_type, resp_payload) = reader.next_frame().unwrap();
        assert_eq!(resp_type, msg::HELLO_RESP);

        let mut server_info = "";
        for (field, value) in FieldIter::new(&resp_payload) {
            if field == 3 {
                server_info = value.as_str();
            }
        }
        assert!(server_info.contains("Test Device"));

        // Send PingRequest
        let frame = frame_plaintext(msg::PING_REQ, &[]);
        client.write_all(&frame).unwrap();

        let n = client.read(&mut buf).unwrap();
        reader.push(&buf[..n]);
        let (resp_type, _) = reader.next_frame().unwrap();
        assert_eq!(resp_type, msg::PING_RESP);

        // Send DisconnectRequest
        let frame = frame_plaintext(msg::DISCONNECT_REQ, &[]);
        client.write_all(&frame).unwrap();

        let n = client.read(&mut buf).unwrap();
        reader.push(&buf[..n]);
        let (resp_type, _) = reader.next_frame().unwrap();
        assert_eq!(resp_type, msg::DISCONNECT_RESP);

        drop(client);
        server.join().unwrap();
    }
}
