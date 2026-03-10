# Della Mini Split AC Units

Della mini split units controlled via the TCL serial protocol, integrated with Home Assistant.

## Approaches

### 1. ESPHome + ESP32-C3 SuperMini (Avoid if possible)

Replaces the stock TCLWBR WiFi module with an ESP32-C3 SuperMini running ESPHome. Integrates natively with Home Assistant via the ESPHome API — no MQTT broker needed. However, this approach requires extra hardware (level shifter), custom wiring, and a more involved installation. Prefer the OpenBeken route unless you have a specific reason to use ESPHome.

- **Hardware:** ESP32-C3 SuperMini
- **Firmware:** ESPHome with custom `tcl_climate` component
- **Protocol:** TCL over UART (9600 baud)
- **Integration:** ESPHome native API → Home Assistant (auto-discovered)

See [ESPHOME_SETUP.md](ESPHOME_SETUP.md) for wiring and flashing details.

### 2. OpenBeken + RTL87x0C (Recommended)

The original stock WiFi module flashed with OpenBeken firmware, controlled via MQTT.

- **Chip:** RTL87x0C (Realtek)
- **Firmware:** OpenBeken
- **Protocol:** TCL (NOT TuyaMCU)
- **Integration:** MQTT → Home Assistant

See [SETUP.md](SETUP.md) for OpenBeken configuration details.

## Quick Reference

### ESPHome (ESP32-C3)

```bash
esphome compile devices/della-minisplits/della-ac.yaml
esphome upload devices/della-minisplits/della-ac.yaml
esphome logs devices/della-minisplits/della-ac.yaml
```

### OpenBeken (Legacy)

```bash
curl "http://<device_ip>/cmd_tool?cmd=startDriver+TCL"
curl "http://<device_ip>/index?state=1"
```

## Features

| Feature | ESPHome | OpenBeken |
|---|---|---|
| Climate modes | off, cool, heat, dry, fan_only, auto | off, cool, heat, dry, fan, heatcool, auto |
| Fan speeds | auto, 1-5, mute, turbo | auto, 1-5, mute, turbo |
| Vertical swing | 9 positions (select entity) | 9 positions (MQTT) |
| Horizontal swing | 10 positions (select entity) | 10 positions (MQTT) |
| Buzzer control | Switch entity | MQTT toggle |
| Display control | Switch entity | MQTT toggle |
| Current temperature | Yes | Yes |
| HA auto-discovery | Native (ESPHome API) | MQTT discovery |
