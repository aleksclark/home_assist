# Status Display

> **Status:** Working — dedicated HVAC dashboard showing thermostat states and room temperatures

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

## Display Layout

```
┌──────────────────────────────────┐
│           HVAC Status            │  header
├──────────────────────────────────┤
│ Kitchen    Cool   72F  68/75    │
│ Livingroom Idle   71F  75F     │
│ Amos BR    Heat   70F  60/68   │
│ Hallway    Idle   73F  67/75   │
├──────────────────────────────────┤
│  A&K BR: 71F     Kitchen: 72F   │  BLE sensors
├──────────────────────────────────┤
│  WiFi OK  |  192.168.0.x        │  footer
└──────────────────────────────────┘
```

### Thermostat Rows

Each row shows:

| Column | Source | Example |
|--------|--------|---------|
| **Name** | Hardcoded label | `Kitchen` |
| **Status** | `hvac_action` attribute | `Cool`, `Heat`, `Idle`, `Off` |
| **Current** | `current_temperature` attribute | `72F` |
| **Range** | `target_temp_low`/`target_temp_high` or `temperature` | `68/75` |

Status text is color-coded: heating→orange, cooling→cyan, idle→gray, off→dim.

### Monitored Entities

| Entity | Type | Purpose |
|--------|------|---------|
| `climate.kitchen_ac` | Della 18k BTU | Kitchen thermostat |
| `climate.livingroom_ac` | Della 12k BTU | Livingroom thermostat |
| `climate.amos_bedroom_ac` | Della 9k BTU | Amos bedroom thermostat |
| `climate.smart_thermostat` | Matter thermostat | Hallway thermostat |
| `sensor.atc_a0c6_temperature` | Xiaomi MiT2 BLE | A&K bedroom measured temp |
| `sensor.kitchen_temperature` | Xiaomi MiT2 BLE | Kitchen measured temp |

## Home Assistant Integration

The device implements the **ESPHome Native API** (plaintext protobuf over TCP on port 6053). No YAML config, no ESPHome compiler — standalone Rust firmware that speaks the same protocol.

### Setup

1. Flash the firmware to the device
2. In HA: **Settings → Devices & Services → Add Integration → ESPHome**
3. Enter the device IP (`192.168.0.105`) and port `6053`
4. The device subscribes to the hardcoded entity list above

### Data Flow

```
HA ──TCP:6053──→ Device (ESPHome API server)

1. HA connects, sends HelloRequest
2. Device responds (no entities to list)
3. Device tells HA which entity_ids + attributes to watch
4. HA pushes state updates in real time
5. Device renders updates on display (500ms refresh)
```

## Firmware Architecture

```
firmware/src/
├── main.rs     Hardware init, WiFi, spawn API server + display loop
├── api.rs      ESPHome native API server (TCP, protobuf)
├── proto.rs    Protobuf wire format: VarInt, field encode/decode, frame I/O
├── state.rs    Hardcoded thermostat + sensor state (SharedState via Arc<Mutex>)
└── ui.rs       HVAC dashboard rendering using ha-display-kit
```

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
