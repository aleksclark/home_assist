# Della Mini Split AC Units

Della mini split units controlled via the TCL serial protocol, integrated with Home Assistant.

## Approaches

### 1. OpenBeken on stock TCLWBR (Current)

The stock TCLWBR WiFi module (RTL87x0C) flashed with OpenBeken firmware, controlled via MQTT. This is what all 3 units are running today.

- **Module:** TCLWBR (RTL87x0C / Realtek)
- **Firmware:** OpenBeken
- **Protocol:** TCL (NOT TuyaMCU)
- **Integration:** MQTT → Home Assistant

See [SETUP.md](SETUP.md) for OpenBeken flashing and configuration details.

### 2. ESPHome + ESP32-C3 SuperMini (Alternative)

Replaces the stock TCLWBR module with an ESP32-C3 SuperMini running ESPHome. Integrates natively with Home Assistant via the ESPHome API — no MQTT broker needed. Adds dual-setpoint `heat_cool` mode support. Requires extra hardware (level shifter), custom wiring, and physical access to each indoor unit.

- **Hardware:** ESP32-C3 SuperMini + BSS138 level shifter
- **Firmware:** ESPHome with custom `tcl_climate` component
- **Protocol:** TCL over UART (9600 baud)
- **Integration:** ESPHome native API → Home Assistant (auto-discovered)

See [ESPHOME_SETUP.md](ESPHOME_SETUP.md) for wiring and flashing details.

## Quick Reference

### OpenBeken (Current)

```bash
curl "http://<device_ip>/cmd_tool?cmd=startDriver+TCL"
curl "http://<device_ip>/index?state=1"
```

### ESPHome (Alternative)

```bash
esphome compile devices/della-minisplits/della-ac.yaml
esphome upload devices/della-minisplits/della-ac.yaml
esphome logs devices/della-minisplits/della-ac.yaml
```

## Features

| Feature | OpenBeken (Current) | ESPHome (Alternative) |
|---|---|---|
| Climate modes | off, cool, heat, dry, fan, auto | off, cool, heat, heat_cool, dry, fan_only, auto |
| Dual setpoint (heat_cool) | No (single setpoint; HA automations handle heat/cool switching) | Yes (firmware-level, dual-slider UI in HA) |
| Fan speeds | auto, 1-5, mute, turbo | auto, 1-5, mute, turbo |
| Vertical swing | 9 positions (MQTT) | 9 positions (select entity) |
| Horizontal swing | 10 positions (MQTT) | 10 positions (select entity) |
| Buzzer control | MQTT toggle | Switch entity |
| Display control | MQTT toggle | Switch entity |
| Current temperature | Yes | Yes |
| HA integration | MQTT climate entity | ESPHome native API (auto-discovered) |
| Hardware change needed | No (flash stock TCLWBR in-place) | Yes (replace TCLWBR with ESP32-C3 + level shifter) |
