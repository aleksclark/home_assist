# Della Mini Split — ESPHome + ESP32-C3 SuperMini Setup

Replace the stock TCLWBR WiFi module with an ESP32-C3 SuperMini running ESPHome. The mini split's main MCU communicates via a 4-wire UART connection using the TCL protocol at 9600 baud. The ESP32-C3 speaks this protocol natively through the custom `tcl_climate` ESPHome component.

## Hardware

- **ESP32-C3 SuperMini** (any variant with USB-C, ~$2-3)
- **Bidirectional logic level shifter** (BSS138-based 2-channel, ~$1) — required because the AC uses 5V UART and the ESP32-C3 is 3.3V logic
- The stock TCLWBR module connector provides **5V, GND, TX, RX**

## Wiring

The Della mini split's control board uses **5V UART logic levels**. The ESP32-C3 is 3.3V. A level shifter is **required** — without it the AC cannot read the ESP's 3.3V TX output, and the AC's 5V TX risks damaging the ESP's GPIO.

```
AC Board (5V)       Level Shifter        ESP32-C3 SuperMini (3.3V)
────────────       ───────────────       ─────────────────────────
  5V  ──────────── HV  ──────────── LV ──── 3.3V
  GND ──────────── GND ──────────── GND ─── GND
  TX  ──────────── HV1 ──────────── LV1 ─── GPIO20 (RX)
  RX  ──────────── HV2 ──────────── LV2 ─── GPIO21 (TX)
  5V  ──────────────────────────────────── 5V (power only)
```

The mini split provides 5V which powers the ESP32-C3 through its 5V pin (onboard regulator steps down to 3.3V). The level shifter's HV side is powered from the AC's 5V, and the LV side from the ESP's 3.3V output.

> **Pin choice:** GPIO20/21 are the default UART pins on ESP32-C3 SuperMini. The YAML config uses these — change them if your board differs.

## Component Structure

```
devices/della-minisplits/
├── della-ac.yaml                          # ESPHome device config
├── components/
│   └── tcl_climate/
│       ├── __init__.py                    # (empty, required by Python)
│       ├── climate.py                     # ESPHome codegen & config schema
│       ├── tcl_climate.h                  # C++ header (protocol structs, class def)
│       └── tcl_climate.cpp                # C++ implementation (protocol logic)
```

## First Flash (USB)

1. Connect the ESP32-C3 SuperMini via USB to your computer
2. Compile and upload:

```bash
esphome compile devices/della-minisplits/della-ac.yaml
esphome upload devices/della-minisplits/della-ac.yaml --device /dev/ttyACM0
```

3. After first flash, subsequent updates go over WiFi (OTA)

## Installation

1. Power off the mini split at the breaker
2. Open the indoor unit's front panel to access the control board
3. Locate the TCLWBR module (small board connected via 4-pin header)
4. Disconnect the TCLWBR module
5. Wire the ESP32-C3 SuperMini to the 4-pin header per the wiring diagram above
6. Secure the ESP32-C3 (hot glue, double-sided tape, or 3D-printed bracket)
7. Power on the mini split

## Home Assistant

The device auto-discovers via ESPHome's native API. No MQTT or `configuration.yaml` changes needed.

### Entities Created

| Entity | Type | Description |
|---|---|---|
| **Della AC** | Climate | Main climate control (mode, temp, fan, swing) |
| **Della Vertical Swing** | Select | Granular vertical vane position (9 options) |
| **Della Horizontal Swing** | Select | Granular horizontal vane position (10 options) |
| **Della Buzzer** | Switch | Toggle the unit's beep on commands |
| **Della Display** | Switch | Toggle the temperature display on the unit |

### Climate Modes

| Mode | Description |
|---|---|
| Off | Unit powered off |
| Cool | Cooling mode |
| Heat | Heating mode |
| Dry | Dehumidification |
| Fan Only | Fan only, no compressor |
| Auto | Automatic mode selection |

### Fan Speeds

`auto`, `1`, `2`, `3`, `4`, `5`, `mute`, `turbo`

### Vertical Swing Positions

`none`, `move_full`, `move_upper`, `move_lower`, `fix_top`, `fix_upper`, `fix_mid`, `fix_lower`, `fix_bottom`

### Horizontal Swing Positions

`none`, `move_full`, `move_left`, `move_mid`, `move_right`, `fix_left`, `fix_mid_left`, `fix_mid`, `fix_mid_right`, `fix_right`

## Monitoring

```bash
esphome logs devices/della-minisplits/della-ac.yaml
```

Look for `[tcl_climate]` log lines showing state updates from the AC unit.

## Troubleshooting

### No communication with AC

- Verify wiring: TX/RX may be swapped. Try swapping GPIO20 and GPIO21
- Check that the mini split is powered on (the control board must be energized)
- Verify 5V is present on the connector with a multimeter
- Check ESPHome logs for any UART data: if you see hex dumps, communication is working but packets may be malformed

### Device shows in HA but no temperature data

- The AC must be polled for ~1 second before data appears
- Check logs for "Bad checksum" warnings — may indicate noise on the UART line
- Ensure baud rate is 9600 and no parity (the YAML config handles this)

### Commands sent but AC doesn't respond

- The component waits for at least one valid response before sending commands
- Verify the AC is not in an error state (check the unit's own display)

## Protocol Notes

The TCL protocol uses a simple request/response pattern:
- **Poll:** 8-byte request packet sent every 500ms
- **Response:** 61-byte status packet from AC MCU (mode, temp, fan, swing, etc.)
- **Set:** 35-byte command packet (built from last known state + desired changes)
- **Framing:** All packets start with `0xBB`, XOR checksum as last byte
- **Baud:** 9600, 8N1

Protocol implementation ported from [OpenBeken's drv_tclAC.c](https://github.com/openshwprojects/OpenBK7231T_App/blob/main/src/driver/drv_tclAC.c).
