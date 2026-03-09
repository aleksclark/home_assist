# AGENTS.md — Home Automation Setup

This repository holds all configuration, firmware, and documentation for a Home Assistant-based home automation system.

## Infrastructure

| Component             | Details                                                    |
|-----------------------|------------------------------------------------------------|
| **Home Assistant**    | Docker on `192.168.0.3`                                    |
| **MQTT Broker**       | Mosquitto (on HA host or nearby)                           |
| **ESPHome**           | CLI or dashboard for compiling/flashing ESP32 devices      |
| **BLE Proxy Network** | ESP32-C3 nodes providing Bluetooth coverage to HA          |

## Repository Structure

```
setup/
├── AGENTS.md                       # this file
├── .gitignore
│
├── esphome/                        # shared ESPHome resources
│   ├── .gitignore                  # ignores .esphome/ build cache, secrets.yaml
│   ├── secrets.yaml                # API keys, OTA passwords, WiFi creds (git-ignored)
│   └── common/                     # reusable YAML packages
│       ├── base.yaml               # logger, api, ota defaults
│       ├── wifi.yaml               # standard WiFi config
│       └── wifi_stable.yaml        # WiFi with ESP32-C3 stability workaround
│
├── devices/                        # one subdirectory per device type / project
│   ├── ble-scanners/               # ESP32-C3 BLE proxy nodes
│   │   ├── README.md
│   │   └── ble-scanner-*.yaml      # per-location ESPHome configs
│   │
│   ├── ble-thermometers/           # Xiaomi Mijia BLE temp/humidity sensors
│   │   ├── README.md
│   │   └── devices.txt             # sensor MACs, BLE keys, tokens
│   │
│   ├── della-minisplits/           # Della AC units (OpenBeken + TCL protocol)
│   │   ├── README.md
│   │   └── SETUP.md                # full flashing & config guide
│   │
│   ├── ecobee/                     # Ecobee thermostat (WIP)
│   │   └── README.md
│   │
│   ├── status-display/             # ESP32-2432S028 TFT display node (planned)
│   │   └── README.md
│   │
│   └── irrigation/                 # moisture sensing & irrigation (planned)
│       ├── README.md
│       ├── indoor/                 # 4-pump houseplant system
│       └── outdoor/                # 4-solenoid drip irrigation
│
├── tools/                          # flashing tools & firmware binaries
│   ├── flasher/
│   │   └── BK7231GUIFlashTool-main/  # BK7231/RTL GUI flash tool
│   └── firmware/
│       ├── OpenBK7231N_1.18.236.rbl
│       └── OpenRTL87X0C_1.18.236.bin
│
└── home-assistant/                 # HA config snippets & docker notes
    └── README.md
```

## Conventions

### Directory Organization

- **`devices/<project>/`** — Each device type or project gets its own directory under `devices/`. ESPHome YAML configs live alongside the project they belong to, not in a central esphome folder.
- **`esphome/common/`** — Shared YAML packages (`!include` targets) for WiFi, API, OTA, etc. to reduce duplication across device configs.
- **`tools/`** — Flashing utilities and firmware binaries that may apply to multiple projects.
- **`home-assistant/`** — HA-side configuration: `configuration.yaml` snippets, automations, dashboard configs.

### Adding a New Device Type

1. Create `devices/<device-name>/`
2. Add a `README.md` documenting hardware, purpose, and status
3. Place ESPHome YAML configs (if applicable) directly in that directory
4. Reference shared packages from `esphome/common/` via `!include`
5. Update this file's structure diagram

### Naming

- ESPHome device configs: `<function>-<location>.yaml` (e.g., `ble-scanner-kitchen.yaml`)
- Directories: lowercase, hyphen-separated (e.g., `della-minisplits`, `status-display`)

### Secrets

All secrets are managed via `esphome/secrets.yaml` (git-ignored). ESPHome configs reference them with `!secret <key>`. Required keys:

| Key             | Purpose                              |
|-----------------|--------------------------------------|
| `wifi_ssid`     | WiFi network name                    |
| `wifi_password`  | WiFi password                        |
| `api_key`       | Home Assistant native API encryption |
| `ota_password`  | Over-the-air update password         |

## ESPHome Commands

```bash
# Compile a device config
esphome compile devices/ble-scanners/ble-scanner-kitchen.yaml

# Upload firmware over WiFi (OTA)
esphome upload devices/ble-scanners/ble-scanner-kitchen.yaml

# Stream device logs
esphome logs devices/ble-scanners/ble-scanner-kitchen.yaml

# Launch ESPHome web dashboard
esphome dashboard esphome/
```

## Current Device Inventory

### Active

| Device               | Type          | Location(s)                                          | Protocol         |
|----------------------|---------------|------------------------------------------------------|------------------|
| BLE Scanners (6x)   | ESP32-C3      | Office, Kitchen, Livingroom, Bedroom, Bathroom, Carport | ESPHome API    |
| BLE Thermometers (4x)| Xiaomi MiT2  | Kitchen, A&K BR, Amos, Red Room                     | BLE via proxies  |
| Della Mini Splits    | RTL87x0C      | _various_                                            | MQTT (TCL)       |

### Work in Progress

| Device          | Type     | Notes                         |
|-----------------|----------|-------------------------------|
| Ecobee          | Thermostat | HA integration setup pending |

### Planned

| Device               | Type             | Notes                                              |
|----------------------|------------------|----------------------------------------------------|
| Status Display       | ESP32-2432S028   | Wall-mounted TFT for status & control              |
| Indoor Irrigation    | ESP32 + 4 pumps  | Capacitive moisture sensing, peristaltic pumps     |
| Outdoor Irrigation   | ESP32 + 4 valves | Capacitive moisture sensing, solenoid drip control |

## Network

- **Home Assistant:** `192.168.0.3`
- **ESPHome devices:** DHCP (recommend static leases in router)
- **MQTT broker:** configured in HA (Settings → Devices & Services → MQTT)
