# Status Display

> **Status:** Working — live HA dashboard via webhook API

ESP32-2432S028 (CYD / Cheap Yellow Display) wall-mounted status display and control node.

## Hardware

| Spec          | Value                                    |
|---------------|------------------------------------------|
| **Module**    | ESP32-WROOM-32 (Xtensa dual-core LX6)   |
| **Clock**     | 240 MHz                                  |
| **SRAM**      | 520 KB                                   |
| **Flash**     | 4 MB                                     |
| **PSRAM**     | None (mod available)                     |
| **Display**   | 2.8" ILI9341 320x240 TFT (SPI)          |
| **Touch**     | XPT2046 resistive (SPI)                  |
| **Extras**    | RGB LED, LDR, SD slot, speaker           |
| **WiFi**      | 802.11 b/g/n                             |

## Architecture Decision

**Rust + esp-idf-hal + mipidsi + embedded-graphics**

### Why Rust

- First-class ESP32 support via `esp-rs` ecosystem (`esp-idf-hal`, `esp-idf-svc`)
- Built-in MQTT client in `esp-idf-svc` (blocking + async)
- WiFi, SPI, GPIO all handled by `esp-idf-hal`
- Memory safety matters on a device with 520KB SRAM
- `std` support via ESP-IDF (not bare-metal `no_std`)

### Why mipidsi + embedded-graphics (not LVGL, not Slint)

| Framework           | GUI widgets | ESP32 (original) | RAM fit    | Rust build complexity |
|---------------------|-------------|-------------------|------------|-----------------------|
| **mipidsi + e-g**   | None        | Excellent         | 2-5 KB     | Simple (pure Rust)    |
| LVGL                | Full        | Excellent         | 10-50 KB   | Hard (C bindgen)      |
| Slint               | Full        | S3/P4 only*       | <300 KB    | N/A for ESP32         |

\* Slint's official examples target ESP32-S3/P4 only. No board support for the original ESP32-WROOM-32.

- **mipidsi** is the display driver used by `esp-idf-hal`'s own SPI display examples — proven, stable, and zero build friction
- LVGL would provide richer widgets (buttons, sliders, charts) but `lv_binding_rust` requires complex C build integration (bindgen, lv_conf.h) that proved unreliable on ESP32
- `embedded-graphics` provides drawing primitives (rectangles, text, lines) — sufficient for a dashboard display
- Touch-interactive widgets will need custom implementation or a future LVGL migration

### Why not Go/TinyGo

TinyGo supports original ESP32 GPIO/SPI/I2C but **does not support WiFi or Bluetooth** — a hard blocker for an MQTT-connected device.

### Why not esp-hal (bare metal no_std)

No WiFi stack. Need `esp-idf` for WiFi + MQTT.

## Toolchain Setup

The ESP32 (Xtensa) requires Espressif's fork of Rust — upstream does not support Xtensa.

```bash
# Install tools
cargo install espup ldproxy cargo-generate
cargo install espflash --version 3.3.0  # v4.3.0 fails to compile (nix crate issue)

# Install Xtensa toolchain
espup install --targets esp32

# Build
# ldproxy + espflash are in ~/.asdf/installs/rust/<ver>/bin/ (asdf) or ~/.cargo/bin/
# The esp toolchain cargo is at ~/.rustup/toolchains/esp/bin/cargo
export RUSTUP_TOOLCHAIN=esp
export LIBCLANG_PATH="$HOME/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
export PATH="$HOME/.asdf/installs/rust/1.93.0/bin:$HOME/.rustup/toolchains/esp/bin:$HOME/.rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin:$PATH"
cd devices/status-display/firmware
cargo build

# Flash
espflash flash -p /dev/ttyUSB0 --chip esp32 target/xtensa-esp32-espidf/debug/status-display

# Monitor (resets device on connect)
espflash monitor -p /dev/ttyUSB0 -e target/xtensa-esp32-espidf/debug/status-display
```

### Key Dependencies

```toml
[dependencies]
esp-idf-hal = "0.45"                # HAL (SPI, GPIO, etc.)
esp-idf-svc = "0.51"                # Services (WiFi, HTTP, NVS)
esp-idf-sys = "0.36"                # Low-level ESP-IDF bindings
mipidsi = "0.8"                     # ILI9341 display driver
display-interface-spi = "0.5"       # SPI display interface
embedded-graphics = "0.8"           # Drawing primitives, text
embedded-hal = "1"                  # Hardware abstraction traits
embedded-svc = "0.28"               # HTTP client traits
serde = "1"                         # JSON deserialization
serde_json = "1"                    # JSON parsing
anyhow = "1"
log = "0.4"
```

### Version Compatibility

These versions are carefully matched — mismatches cause trait incompatibilities:

- `esp-idf-hal 0.45` → `embedded-hal 1.0` + `embedded-graphics-core 0.4`
- `mipidsi 0.8` → `embedded-hal 1.0` + `display-interface 0.5` + `embedded-graphics-core 0.4` ✓
- `mipidsi 0.7` → `embedded-hal 0.2` + `display-interface 0.4` ✗ (incompatible)

## Home Assistant Integration

The display registers as a **mobile_app device** with Home Assistant, using the native webhook API. This means:

- **First boot only**: requires a one-time long-lived access token (generated in HA: Profile → Long-Lived Access Tokens)
- Registration creates a persistent `webhook_id` stored in NVS flash
- **All subsequent boots**: uses the webhook URL with **no authentication required**
- If the HA integration is deleted (HTTP 410), the device must re-register

### Setup

1. Generate a long-lived access token in HA (Profile → Long-Lived Access Tokens)
2. Set `HA_PROVISION_TOKEN` in `firmware/src/main.rs`
3. Flash and boot the device — it registers automatically
4. The device appears in HA under Settings → Devices as "Status Display"
5. After first boot succeeds, the token can be removed from the source

### Data Flow

Polls HA every 30 seconds via `POST /api/webhook/<id>` with `render_template`:

| Entity                                 | Display Card | Data        |
|----------------------------------------|--------------|-------------|
| `sensor.kitchen_temp_temperature`      | Kitchen      | Temperature |
| `sensor.kitchen_temp_humidity`         | Kitchen      | Humidity    |
| `sensor.atc_a0c6_temperature`         | Bedroom      | Temperature |
| `sensor.atc_a0c6_humidity`            | Bedroom      | Humidity    |
| `climate.della_mini_split`             | Della        | Mode, target, fan |
| `climate.my_ecobee`                    | Ecobee       | Mode        |
| `sensor.my_ecobee_current_temperature` | Ecobee       | Temperature |
| `sensor.my_ecobee_current_humidity`    | Ecobee       | Humidity    |

## Display Layout

```
┌─────────────────────────────────┐
│          Home Status            │  <- header bar
├────────────────┬────────────────┤
│  Kitchen       │  Bedroom       │  <- BLE thermometer readings
│  74 F          │  73 F          │
│  65% RH        │  67% RH        │
├────────────────┴────────────────┤
│  Della Mini Split               │  <- mini split status
│  Auto  70 F       Fan: 5       │
├─────────────────────────────────┤
│  Ecobee: unavailable            │  <- ecobee status
├─────────────────────────────────┤
│  WiFi OK  |  192.168.0.x        │  <- footer
└─────────────────────────────────┘
```

## Pin Mapping

| Function       | Pin(s)                              |
|----------------|-------------------------------------|
| TFT MOSI       | GPIO 13                             |
| TFT SCLK       | GPIO 14                             |
| TFT CS         | GPIO 15                             |
| TFT DC         | GPIO 2                              |
| TFT Backlight  | GPIO 21                             |
| Touch MOSI     | GPIO 32                             |
| Touch MISO     | GPIO 39                             |
| Touch CLK      | GPIO 25                             |
| Touch CS       | GPIO 33                             |
| Touch IRQ      | GPIO 36                             |
