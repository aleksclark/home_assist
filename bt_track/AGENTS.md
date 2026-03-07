# AGENTS.md - BT Track Project

This repository contains configuration and tools for Bluetooth tracking and IoT device management using ESPHome and OpenBeken firmware.

## Project Overview

A home automation project focused on:
- **BLE (Bluetooth Low Energy) scanning** via ESP32-C3 devices running ESPHome
- **Xiaomi temperature/humidity sensors** (miaomiaoce.sensor_ht.t2) monitoring
- **WiFi module flashing** for Tuya/Beken devices using OpenBeken firmware
- **Mini-split AC control** via TCL protocol on RTL87x0C chips

## Directory Structure

```
bt_track/
├── esphome/               # ESPHome device configurations
│   ├── ble-scanner*.yaml  # BLE proxy nodes for different rooms
│   ├── secrets.yaml       # API keys and OTA passwords
│   └── .esphome/          # ESPHome build artifacts and storage
├── flasher/               # Firmware flashing tools
│   └── BK7231GUIFlashTool-main/  # Windows GUI tool for Beken/RTL chips
├── wbr1_flash/            # OpenBeken firmware binaries
├── devices.txt            # Xiaomi BLE device credentials (MAC, keys, tokens)
└── DELLA_SETUP.md         # Della mini-split AC setup guide
```

## ESPHome Configuration

### Hardware
- **Board:** ESP32-C3-DevKitM-1
- **Framework:** ESP-IDF (not Arduino)
- **Function:** Bluetooth proxy for Home Assistant

### Device Naming Convention
```
ble-scanner-<location>.yaml
```
Locations: office, bathroom, bedroom, kitchen, livingroom, carport

### Common Configuration Pattern
```yaml
esphome:
  name: ble-scanner-<location>
  friendly_name: BLE Scanner <Location>

esp32:
  board: esp32-c3-devkitm-1
  framework:
    type: esp-idf

esp32_ble_tracker:
  scan_parameters:
    interval: 1100ms
    window: 1100ms
    active: true

bluetooth_proxy:
  active: true
```

### WiFi Stability Workaround
Some devices include a WiFi protocol fix for ESP32-C3 stability:
```yaml
esphome:
  includes:
    - <esp_wifi.h>
  on_boot:
    - priority: 300
      then:
        lambda: |-
          esp_wifi_set_protocol(WIFI_IF_STA, WIFI_PROTOCOL_11B|WIFI_PROTOCOL_11G);

wifi:
  power_save_mode: none
  fast_connect: On
  output_power: 8.5
```
This forces 802.11b/g only and sets conservative power settings.

### Secrets Management
All devices reference `secrets.yaml` for:
- `api_key` - Home Assistant API encryption key
- `ota_password` - Over-the-air update password

### ESPHome Commands
```bash
# Compile firmware
esphome compile esphome/ble-scanner-<location>.yaml

# Upload to device
esphome upload esphome/ble-scanner-<location>.yaml

# View logs
esphome logs esphome/ble-scanner-<location>.yaml

# Run ESPHome dashboard (web UI)
esphome dashboard esphome/
```

## BLE Device Tracking

### Supported Sensors
- **Model:** Xiaomi Mijia Temperature/Humidity Sensor (miaomiaoce.sensor_ht.t2)
- **Protocol:** ATC MiThermometer (custom firmware) or native Xiaomi BLE

### Device Credentials
`devices.txt` contains for each sensor:
- `NAME` - Location identifier
- `MAC` - Bluetooth MAC address (A4:C1:38:xx:xx:xx)
- `BLE KEY` - 32-char hex encryption key
- `TOKEN` - 24-char hex token
- `ID` - Cloud device ID

### Sensor Configuration Example
```yaml
sensor:
  - platform: atc_mithermometer
    mac_address: "A4:C1:38:92:48:AF"
    temperature:
      name: "Room Temperature"
    humidity:
      name: "Room Humidity"
    battery_level:
      name: "Room Battery"
```

## OpenBeken / Firmware Flashing

### Supported Chips
- Beken: BK7231T, BK7231N, BK7231M, BL2028N, BK7238
- Realtek: RTL8710B, RTL8710C, RTL8720D/RTL8720CS
- Others: BL602, ECR6600, LN882H, RDA5981, W800, W600

### Flashing Tool
Located at `flasher/BK7231GUIFlashTool-main/`

**Windows:**
```
BK7231Flasher.exe
```

**Linux (Mono):**
```bash
msbuild BK7231Flasher/BK7231Flasher.csproj
mono BK7231Flasher/bin/debug/BK7231Flasher.exe
```

### Flashing Process (BK72xx)
1. Connect UART-to-USB adapter (3.3V) to TXD1/RXD1
2. Select chip type (N or T)
3. Download latest firmware from web
4. Click "Backup and flash new"
5. Reset device when "Getting bus" appears

### Available Firmware
`wbr1_flash/` contains:
- `OpenBK7231N_1.18.236.rbl` - Beken BK7231N firmware
- `OpenRTL87X0C_1.18.236.bin` - Realtek RTL87x0C firmware

## Della Mini-Split AC Setup

Uses TCL protocol (NOT TuyaMCU) on RTL87x0C chips. See `DELLA_SETUP.md` for full details.

### Quick Commands
```bash
# Start TCL driver
curl "http://<device_ip>/cmd_tool?cmd=startDriver+TCL"

# Save to startup
curl "http://<device_ip>/startup_command?startup_cmd=1&data=startDriver%20TCL"

# Check status
curl "http://<device_ip>/index?state=1"
```

### MQTT Topics
- State: `<device>/ACMode/get`, `<device>/TargetTemperature/get`, etc.
- Commands: `cmnd/<device>/ACMode`, `cmnd/<device>/TargetTemperature`, etc.

## Important Notes

### Security
- `secrets.yaml` contains sensitive credentials - never commit to public repos
- WiFi passwords are hardcoded in YAML files - consider using secrets

### Known Issues
1. **Livingroom typo:** `ble-scanner-livingroom.yaml` has `name: ble-scanner-living-toom` (typo)
2. **ESP32-C3 WiFi instability:** Some devices need the 802.11b/g workaround
3. **Della AC:** Uses TCL protocol, NOT TuyaMCU - empty channels indicate wrong driver

### File Locations
- ESPHome build cache: `esphome/.esphome/`
- Flasher backups: `flasher/BK7231GUIFlashTool-main/backups/`
- Test dumps: `flasher/BK7231GUIFlashTool-main/BK7231Flasher/testDumps/`

## External Resources

- [ESPHome Documentation](https://esphome.io/)
- [OpenBeken GitHub](https://github.com/openshwprojects/OpenBK7231T_App)
- [BK7231 Flasher Tool](https://github.com/openshwprojects/BK7231GUIFlashTool)
- [Elektroda Forum](https://www.elektroda.com/) - Support for OpenBeken
