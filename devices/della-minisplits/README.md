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

## Patched OpenBeken firmware

All three units run upstream OpenBeken plus `openbeken-heat-cool.patch`, built
against a pinned upstream commit by `build-firmware.sh` (Docker, RTL87X0C target).

```bash
./build-firmware.sh                       # -> firmware/OpenRTL87X0C_<ver>_heat_cool_ota.img
# OTA: POST the _ota.img to the unit, then reboot
python3 -c "import urllib.request as u; u.urlopen(u.Request('http://<ip>/api/ota', data=open('firmware/<img>','rb').read(), method='POST'), timeout=180)"
python3 -c "import urllib.request as u; u.urlopen(u.Request('http://<ip>/api/reboot', data=b'', method='POST'))"
mosquitto_pub -h <broker> -t cmnd/<client_id>/scheduleHADiscovery -m 1   # refresh HA discovery
```

The simulator unit tests (`src/selftest/selftest_tclAC.c`) run inside the same
Docker image: `make -f custom.mk && ./build/win_main -runUnitTests 2` (as `-u 0`
under rootless Docker).

### MQTT topics (`<client_id>/<Name>/get`)

| Topic | Values | Notes |
|---|---|---|
| `CurrentTemperature` | °C (int) | |
| `TargetTempLow` / `TargetTempHigh` | °C (int) | heat_cool setpoints, persisted to flash |
| `ACMode` | `off`, `heat_cool`, `fan_only` | |
| `FANMode` | `1`, `2`, `3`, `auto` | |
| `HVACAction` | `heating`, `cooling`, `drying`, `defrosting`, `fan`, `idle`, `off` | real operating state, wired to the climate entity's `act_t` |
| `FanRunning` | `1` / `0` | indoor fan actually spinning; HA `binary_sensor` (device class `running`) |
| `Buzzer`, `Display`, `SwingH`, `SwingV` | | |

Values publish immediately on change; the periodic refresh is staggered to one
topic per second.

### Operating state decoding

In `heat_cool` the hardware sits in heat or cool mode and its own thermostat
cycles the compressor, so the mode says nothing about whether it is actually
running. `HVACAction`/`FanRunning` come from bytes of the 61-byte `0x04` status
frame mapped on the livingroom unit (forced heat/cool cycles and fan speed changes):

| Byte | Meaning | Observed |
|---|---|---|
| 34 | indoor fan actual speed | 0 = stopped (heat warm-up / heat idle), ~0x50 mute, ~0x58 auto, ~0x84 turbo |
| 38 | compressor frequency | ramps to ~0x50; lags ~30 s behind a stop, so diagnostic only |
| 39 | outdoor fan (likely) | ramps with compressor, 0 when stopped |
| 40 | outdoor unit flags | 0x0A set while the compressor runs (heat 0xCA, cool 0x8A); 0x40 = reversing valve on (heat side) |

Action = `off` if power is off; else, while byte 40 has 0x0A set: `drying` in
dry mode, `heating` if the valve bit is set, `defrosting` if in heat mode with
the valve off (inferred, not yet observed), otherwise `cooling`; when stopped:
`fan` if byte 34 is non-zero, else `idle`. Direction comes from the valve
rather than the mode because heat_cool can flip the mode while the outdoor unit
is still winding down. Byte 38 and 40 are shown on the device's `/index` page.

### Memory constraints (RTL87x0C)

- Every MQTT queue slot is a ~1.7 KB heap block that is never freed, and HA
  discovery queues all entities at once. The fan `binary_sensor` discovery is
  therefore deferred until the queue drains so it reuses an existing slot.
- Keep the climate discovery JSON small (short keys, 4 fan modes, 2 swing
  modes, no horizontal swing). It is ~960 bytes; the hard limit is 1511.
- Never publish every topic in one tick: bursts overflow the lwIP TCP send
  buffer (`ERR_MEM` -> forced MQTT reconnect).

## Features

| Feature | OpenBeken (Current) | ESPHome (Alternative) |
|---|---|---|
| Climate modes | off, heat_cool, fan_only | off, cool, heat, heat_cool, dry, fan_only, auto |
| Dual setpoint (heat_cool) | Yes (firmware-level, persisted to flash) | Yes (firmware-level, dual-slider UI in HA) |
| HVAC action (heating/cooling/fan/idle) | Yes (from compressor/fan state) | Yes (from commanded mode) |
| Indoor fan running | Yes (binary_sensor) | No |
| Fan speeds | auto, 1 (mute), 2, 3 (turbo) | auto, 1-5, mute, turbo |
| Vertical swing | none, move_full | 9 positions (select entity) |
| Horizontal swing | MQTT only (not in discovery) | 10 positions (select entity) |
| Buzzer control | Switch entity | Switch entity |
| Display control | Switch entity | Switch entity |
| Current temperature | Yes | Yes |
| HA integration | MQTT discovery | ESPHome native API (auto-discovered) |
| Hardware change needed | No (flash stock TCLWBR in-place) | Yes (replace TCLWBR with ESP32-C3 + level shifter) |
