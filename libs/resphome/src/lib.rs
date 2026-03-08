pub mod proto;
pub mod api;
pub mod entity;
pub mod wifi;
pub mod ble;
pub mod ota;
pub mod sensor;
pub mod device;

pub use api::{ApiServer, ApiConfig, ClientHandler};
pub use device::{DeviceConfig, DeviceInfo};
pub use entity::{Entity, EntityKind, EntityState, SensorClass, SensorEntity, TextSensorEntity, BinarySensorEntity, SelectEntity};
pub use proto::{encode_varint, decode_varint, frame_plaintext, FrameReader, FieldIter, FieldValue};
pub use sensor::{SensorReading, SensorPlatform};
pub use wifi::WifiConfig;
pub use ble::{BleConfig, BleScanner, BleDevice, BleProxyMode};
pub use ota::OtaConfig;
