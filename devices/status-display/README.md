# Status Display

> **Status:** Working — ESPHome-compatible device with HA-configurable display slots

ESP32-2432S028 (CYD / Cheap Yellow Display) wall-mounted status display. Acts as an ESPHome native API device — Home Assistant connects to it and pushes entity state updates in real time.

## Hardware

| Spec          | Value                                    |
|---------------|------------------------------------------|
| **Module**    | ESP32-WROOM-32 (Xtensa dual-core LX6)   |
| **Clock**     | 240 MHz                                  |
| **SRAM**      | 520 KB                                   |
| **Flash**     | 4 MB                                     |
| **Display**   | 2.8" ILI9341 320×240 TFT (SPI)          |
| **Touch**     | XPT2046 resistive (SPI)                  |
| **WiFi**      | 802.11 b/g/n                             |

## Home Assistant Integration

The device implements the **ESPHome Native API** (plaintext protobuf over TCP on port 6053). No YAML config, no ESPHome compiler — just a standalone Rust firmware that speaks the same protocol.

### Setup

1. Flash the firmware to the device
2. In HA: **Settings → Devices & Services → Add Integration → ESPHome**
3. Enter the device IP (`192.168.0.105`) and port `6053`
4. The device appears with 30 configurable entities (5 per display slot × 6 slots)

### Configurable Display Slots

Each of the 6 display slots exposes 5 Select entities in HA:

| Entity | Purpose | Example |
|--------|---------|---------|
| **Slot N Entity ID** | HA entity to subscribe to | `sensor.kitchen_temp_temperature` |
| **Slot N Label** | Display label text | `Kitchen` |
| **Slot N Display Type** | `numeric`, `text`, or `status` | `numeric` |
| **Slot N Unit** | Unit suffix for numeric values | `°F` |
| **Slot N Attribute** | Entity attribute (blank = state) | `current_temperature` |

### Display Types

- **numeric** — Parses value as float, displays with unit suffix. Color: green.
- **text** — Displays raw string value (truncated to 20 chars). Color: green.
- **status** — Maps value to color: `heat`→orange, `cool`→cyan, `auto`→green, `off`→dim, `on`/`home`/`active`→green, `idle`→gray, etc.

### Data Flow

```
HA ──TCP:6053──→ Device (ESPHome API server)

1. HA connects, sends HelloRequest
2. Device responds with entity list (30 Select + dynamic Sensor/TextSensor)
3. HA subscribes to states
4. Device tells HA which entity_ids to watch (SubscribeHomeAssistantStateResponse)
5. HA pushes state updates in real time (HomeAssistantStateResponse)
6. Device renders updates on display with incremental redraw (500ms refresh)
```

## Display Layout

```
┌─────────────────────────────────┐
│          Home Status            │  ← header bar
├────────────────┬────────────────┤
│  Slot 1        │  Slot 2        │  ← configurable
│  72 F          │  73 F          │
├────────────────┼────────────────┤
│  Slot 3        │  Slot 4        │  ← configurable
│  Auto          │  65% RH        │
├────────────────┼────────────────┤
│  Slot 5        │  Slot 6        │  ← configurable
├────────────────┴────────────────┤
│  WiFi OK  |  192.168.0.x        │  ← footer
└─────────────────────────────────┘
```

## Firmware Architecture

```
firmware/src/
├── main.rs     Hardware init, WiFi, spawn API server + display loop
├── api.rs      ESPHome native API server (TCP, protobuf, entity management)
├── proto.rs    Protobuf wire format: VarInt, field encode/decode, frame I/O
├── slots.rs    Configurable display slots (shared state via Arc<Mutex>)
└── ui.rs       Dynamic rendering using ha-display-kit library
```

### ESPHome API Compliance

Implements the subset of the ESPHome Native API needed for a display device:

| Message | ID | Direction | Supported |
|---------|----|-----------|-----------|
| HelloRequest/Response | 1/2 | Both | ✓ |
| DeviceInfoRequest/Response | 9/10 | Both | ✓ |
| ListEntitiesRequest/Done | 11/19 | Both | ✓ |
| ListEntitiesSelectResponse | 52 | Server→Client | ✓ |
| ListEntitiesSensorResponse | 16 | Server→Client | ✓ |
| ListEntitiesTextSensorResponse | 18 | Server→Client | ✓ |
| SubscribeStatesRequest | 20 | Client→Server | ✓ |
| SelectStateResponse | 53 | Server→Client | ✓ |
| SensorStateResponse | 25 | Server→Client | ✓ |
| TextSensorStateResponse | 27 | Server→Client | ✓ |
| SelectCommandRequest | 54 | Client→Server | ✓ |
| SubscribeHomeAssistantStatesReq | 38 | Client→Server | ✓ |
| SubscribeHomeAssistantStateResp | 39 | Server→Client | ✓ |
| HomeAssistantStateResponse | 40 | Client→Server | ✓ |
| PingRequest/Response | 7/8 | Both | ✓ |
| DisconnectRequest/Response | 5/6 | Both | ✓ |
| GetTimeRequest/Response | 36/37 | Both | ✓ |

## Toolchain Setup

The ESP32 (Xtensa) requires Espressif's fork of Rust.

```bash
cargo install espup ldproxy cargo-generate
cargo install espflash --version 3.3.0

espup install --targets esp32

export RUSTUP_TOOLCHAIN=esp
export LIBCLANG_PATH="$HOME/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
export PATH="$HOME/.asdf/installs/rust/1.93.0/bin:$HOME/.rustup/toolchains/esp/bin:$HOME/.rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin:$PATH"

cd devices/status-display/firmware
cargo build

espflash flash -p /dev/ttyUSB0 --chip esp32 target/xtensa-esp32-espidf/debug/status-display
```

## Pin Mapping

| Function       | Pin(s)  |
|----------------|---------|
| TFT MOSI       | GPIO 13 |
| TFT SCLK       | GPIO 14 |
| TFT CS         | GPIO 15 |
| TFT DC         | GPIO 2  |
| TFT Backlight  | GPIO 21 |
| Touch MOSI     | GPIO 32 |
| Touch MISO     | GPIO 39 |
| Touch CLK      | GPIO 25 |
| Touch CS       | GPIO 33 |
| Touch IRQ      | GPIO 36 |
