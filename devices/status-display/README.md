# Status Display

> **Status:** Working — dedicated HVAC dashboard showing real heating/cooling/fan state and room temperatures

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

Flat dark dashboard designed in Google Stitch ("HVAC 320x240 Wall Dashboard")
and reproduced pixel-for-pixel in firmware:

```
┌───────────────┬───────────────┐
│ KITCHEN   FAN │ LIVING ROOM   │  2x2 room tiles (154x92)
│ 70°F          │ 71°F          │  big current temp
│ • HEATING 66-73° • COOLING …  │  real action + setpoint range
├───────────────┼───────────────┤
│ AMOS BR       │ HALLWAY   FAN │
│ 68°F          │ 67°F          │
│ • IDLE  64-72°│ • FAN ONLY …  │
├───────────────┴───────────────┤
│ Band 66-74°     A&K BR 71°  Kitchen 72° │  info strip
│ • BB: Working              • 192.168.0.88 │
└───────────────────────────────┘
```

- **Left accent bar + action label** come from the climate entity's `hvac_action`
  (the minisplits report real compressor state): heating (orange), cooling (blue),
  fan only (teal), drying (yellow), defrost (violet), idle/auto (gray), off (dim).
- **FAN tag** shows when the indoor fan is actually spinning (minisplit `Fan`
  binary_sensors; for the hallway it is implied by a fan-only action).
- **Connection dot** is green with the IP while Home Assistant is connected,
  red "HA offline" otherwise (data on screen may then be stale).
- Touch is whole-screen only (resistive panel): any tap wakes the backlight;
  there are no on-screen controls.

### Monitored Entities

| Entity | Purpose |
|--------|---------|
| `climate.minisplit_kitchen_smart_thermostat` | Kitchen Della 18k |
| `climate.minisplit_livingroom_smart_thermostat` | Livingroom Della 12k |
| `climate.minisplit_amos_smart_thermostat` | Amos bedroom Della 9k |
| `climate.smart_thermostat` | Hallway Matter thermostat |
| `binary_sensor.{kitchen_minisplit_kitchen,living_room_minisplit_livingroom,amos_bedroom_minisplit_amos}_fan` | Indoor fan running |
| `sensor.atc_a0c6_temperature`, `sensor.kitchen_temperature` / `sensor.kitchen_temp_temperature` | BLE room sensors |
| `input_number.hvac_*_{heat,cool}_f` | Schedule band for the current time slot |
| `input_text.bb_status` | Presence status |

## Home Assistant Integration

The device implements the **ESPHome Native API** (plaintext protobuf over TCP on port 6053). No YAML config, no ESPHome compiler — standalone Rust firmware that speaks the same protocol.

### Setup

1. Flash the firmware to the device
2. In HA: **Settings → Devices & Services → Add Integration → ESPHome**
3. Enter the device IP (currently `192.168.0.88`, DHCP) and port `6053`
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
├── main.rs      Hardware init, WiFi, SNTP, API server thread, display loop
├── api.rs       ESPHome native API server (TCP, protobuf), HA session tracking
├── proto.rs     Protobuf wire format: VarInt, field encode/decode, frame I/O
├── state.rs     Entity state (SharedState via Arc<Mutex>), action/fan parsing
├── ui.rs        Dashboard layout/rendering (no ESP-IDF deps)
├── canvas.rs    Off-screen RGB565 framebuffer; regions are pushed in one transfer
├── aa_text.rs   Anti-aliased text renderer (alpha-blends into the canvas)
├── aa_fonts.rs  @generated Roboto glyph bitmaps (tools/gen_aa_fonts.py)
├── ota.rs       Raw OTA receiver (see ota_flash.py)
├── touch.rs / backlight.rs
```

Each tile / the info strip is redrawn only when its content changes, composed
off-screen (one 154x92 buffer, ~28 KB) so updates never flicker. Free heap is
logged every 60 s (`heap: free=… min_free=…`).

### Previewing without hardware

`preview/` compiles `ui.rs`, `state.rs`, `canvas.rs` and the font modules
unchanged for the host and renders sample scenarios to PNG:

```bash
cd devices/status-display/preview
cargo test                      # layout/redraw invariants
cargo run --release -- out/     # out/{design,edge,offline}{,@3x}.png, boot@3x.png
```

### Fonts

Text uses Roboto (Apache 2.0) pre-rasterized with 8-bit anti-aliasing. To change
sizes/weights edit `tools/gen_aa_fonts.py` and regenerate:

```bash
python3 tools/gen_aa_fonts.py /usr/share/fonts/TTF > firmware/src/aa_fonts.rs
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
cargo build --release
# USB (erase otadata so the freshly written ota_0 slot boots):
espflash flash -p /dev/ttyUSB0 --chip esp32 --partition-table partitions.csv \
  --erase-parts otadata target/xtensa-esp32-espidf/release/status-display
# or over WiFi:
espflash save-image --chip esp32 --partition-table partitions.csv \
  target/xtensa-esp32-espidf/release/status-display /tmp/status-display.bin
python3 ota_flash.py <device-ip> /tmp/status-display.bin
```

Opening the USB serial port resets the board.

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
