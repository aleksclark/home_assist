# Della Mini Split AC - OpenBeken Setup Guide

## Overview

Della mini split AC units with built-in WiFi use the **TCL protocol**, NOT TuyaMCU. The WiFi module is typically an RTL87x0C-based chip that can be flashed with OpenBeken firmware.

## Hardware

- **Module:** WBR1 (Tuya-based, RTL8720C / AmebaZ2)
- **Chip:** RTL87x0C (Realtek)
- **Protocol:** TCL (serial communication with AC MCU)
- **Default Device Name:** `rtl87x0C0D1773DB` (based on MAC address)

## Flashing OpenBeken

### Prerequisites

- USB-to-UART adapter (3.3V logic)
- `ltchiptool` (`pip install ltchiptool`)
- OpenBeken firmware: `OpenRTL87X0C_*.bin` (in `tools/firmware/`)
- Separate 3.3V power supply recommended (the UART adapter's regulator may brown out during flash operations)

### WBR1 Wiring

```
--------+        +---------------------
     PC |        | WBR1 (RTL8720C)
--------+        +---------------------
     RX | ------ | TX2 (Log_TX / PA16)
     TX | ------ | RX2 (Log_RX / PA15)
        |        |
    GND | ------ | GND
--------+        +---------------------
```

### Entering Download Mode

The WBR1 uses **PA00** as the download mode strapping pin. To enter download mode:

1. Pull **PA00 HIGH (3.3V)** — this is the opposite of ESP32/ESP8266 where you pull GPIO0 low
2. Ensure **PA13 (RX0) is NOT pulled to GND**
3. Power-cycle the module (or briefly short CEN to GND then release)
4. The chip should now be in download mode

> **WARNING:** The WBR1 module has multiple pads that can be confused for PA00.
> Consult the WBR1 datasheet/pinout diagram and verify you are pulling the
> **correct** pin high. Pulling the wrong pin will cause the chip to boot
> normally or hang, and `ltchiptool` will report "Timeout while linking".
> If you see boot log output (`== Rtl8710c IoT Platform ==`) on the serial
> console, the chip is NOT in download mode.

### Flash Commands

```bash
# Backup existing firmware (recommended)
ltchiptool flash read -d /dev/ttyUSB0 ambz2 backup_wbr1.bin

# Flash OpenBeken
ltchiptool flash write -d /dev/ttyUSB0 tools/firmware/OpenRTL87X0C_1.18.236.bin
```

After flashing, remove the PA00 pull-up and power-cycle to boot normally.

### Troubleshooting Flashing

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Timeout while linking" | Wrong pin pulled high, or not in download mode | Verify PA00 is correct pin per WBR1 pinout diagram |
| Boot log on serial (`== Rtl8710c ==`) | PA00 not pulled high | Pull PA00 to 3.3V before powering on |
| `[SPIF Err]Invalid ID` in boot log | Normal — ROM doesn't recognize flash JEDEC ID, but flashing still works | Ignore and proceed with ltchiptool |
| I/O errors / adapter disconnects | USB-UART adapter can't supply enough current | Use separate 3.3V PSU for the module |
| `OSError: Device or resource busy` | Another process (e.g., `screen`) has the port open | Close other serial connections first |

## OpenBeken Configuration

### 1. Configure WiFi

After flashing, the module starts a WiFi AP named `OpenBeken_XXXXXX` (since no WiFi credentials are configured yet). Connect to this AP from your phone or laptop, then open the web interface at `http://192.168.4.1` to enter your home WiFi SSID and password. Once saved, the module reboots and connects to your network. If the module ever loses its configured network (e.g., credentials change), it will fall back to broadcasting the AP again so you can reconfigure.

### 2. Start TCL Driver

### 2. Start TCL Driver

Via web interface command tool (`http://<device_ip>/cmd_tool`):

```
startDriver TCL
```

### 3. Save to Startup

Save the driver to startup command (`http://<device_ip>/startup_command`):

```
startDriver TCL
```

Or via URL:
```
http://<device_ip>/startup_command?startup_cmd=1&data=startDriver%20TCL
```

### 4. Verify Communication

Check the device index page (`http://<device_ip>/index?state=1`). You should see:
- `1 drivers active (TCL)`
- Mode, Current temperature, Target temperature values
- AC control options (ACMode, SwingV, SwingH)

## MQTT Topics

### State Topics (device publishes)

| Topic | Description | Values |
|-------|-------------|--------|
| `<device>/connected` | Availability | `online` / `offline` |
| `<device>/ACMode/get` | Current mode | `off`, `cool`, `heat`, `dry`, `fan`, `auto` |
| `<device>/CurrentTemperature/get` | Room temperature | Integer (Celsius) |
| `<device>/TargetTemperature/get` | Set temperature | Integer (Celsius) |
| `<device>/FANMode/get` | Fan speed | `1`-`5`, `mute`, `turbo`, `auto` |
| `<device>/SwingV/get` | Vertical swing | `none`, `move_full`, `move_upper`, `move_lower`, `fix_top`, `fix_upper`, `fix_mid`, `fix_lower`, `fix_bottom` |
| `<device>/SwingH/get` | Horizontal swing | `none`, `move_full`, `move_left`, `move_mid`, `move_right`, `fix_left`, `fix_mid_left`, `fix_mid`, `fix_mid_right`, `fix_right` |
| `<device>/Buzzer/get` | Buzzer state | `0` / `1` |
| `<device>/Display/get` | Display state | `0` / `1` |

### Command Topics (to control AC)

Use the `cmnd/` prefix for commands:

| Topic | Description |
|-------|-------------|
| `cmnd/<device>/ACMode` | Set mode |
| `cmnd/<device>/TargetTemperature` | Set temperature |
| `cmnd/<device>/FANMode` | Set fan speed |
| `cmnd/<device>/SwingV` | Set vertical swing |
| `cmnd/<device>/SwingH` | Set horizontal swing |

## Home Assistant Configuration

Add to `configuration.yaml`:

```yaml
mqtt:
  climate:
    - name: "Livingroom AC"
      unique_id: "livingroom_ac"
      temperature_unit: C
      modes:
        - "off"
        - "cool"
        - "heat"
        - "dry"
        - "fan_only"
        - "auto"
      fan_modes:
        - "1"
        - "2"
        - "3"
        - "4"
        - "5"
        - "mute"
        - "turbo"
        - "auto"
      swing_modes:
        - "none"
        - "move_full"
        - "move_upper"
        - "move_lower"
        - "fix_top"
        - "fix_upper"
        - "fix_mid"
        - "fix_lower"
        - "fix_bottom"
      mode_command_topic: "cmnd/<livingroom_device>/ACMode"
      mode_state_topic: "<livingroom_device>/ACMode/get"
      mode_state_template: "{{ value if value != 'fan' else 'fan_only' }}"
      temperature_command_topic: "cmnd/<livingroom_device>/TargetTemperature"
      temperature_state_topic: "<livingroom_device>/TargetTemperature/get"
      current_temperature_topic: "<livingroom_device>/CurrentTemperature/get"
      fan_mode_command_topic: "cmnd/<livingroom_device>/FANMode"
      fan_mode_state_topic: "<livingroom_device>/FANMode/get"
      swing_mode_command_topic: "cmnd/<livingroom_device>/SwingV"
      swing_mode_state_topic: "<livingroom_device>/SwingV/get"
      min_temp: 16
      max_temp: 30
      temp_step: 1
      availability_topic: "<livingroom_device>/connected"
      payload_available: "online"
      payload_not_available: "offline"

    - name: "Kitchen AC"
      unique_id: "kitchen_ac"
      temperature_unit: C
      modes:
        - "off"
        - "cool"
        - "heat"
        - "dry"
        - "fan_only"
        - "auto"
      fan_modes:
        - "1"
        - "2"
        - "3"
        - "4"
        - "5"
        - "mute"
        - "turbo"
        - "auto"
      swing_modes:
        - "none"
        - "move_full"
        - "move_upper"
        - "move_lower"
        - "fix_top"
        - "fix_upper"
        - "fix_mid"
        - "fix_lower"
        - "fix_bottom"
      mode_command_topic: "cmnd/<kitchen_device>/ACMode"
      mode_state_topic: "<kitchen_device>/ACMode/get"
      mode_state_template: "{{ value if value != 'fan' else 'fan_only' }}"
      temperature_command_topic: "cmnd/<kitchen_device>/TargetTemperature"
      temperature_state_topic: "<kitchen_device>/TargetTemperature/get"
      current_temperature_topic: "<kitchen_device>/CurrentTemperature/get"
      fan_mode_command_topic: "cmnd/<kitchen_device>/FANMode"
      fan_mode_state_topic: "<kitchen_device>/FANMode/get"
      swing_mode_command_topic: "cmnd/<kitchen_device>/SwingV"
      swing_mode_state_topic: "<kitchen_device>/SwingV/get"
      min_temp: 16
      max_temp: 30
      temp_step: 1
      availability_topic: "<kitchen_device>/connected"
      payload_available: "online"
      payload_not_available: "offline"

    - name: "Amos Bedroom AC"
      unique_id: "amos_bedroom_ac"
      temperature_unit: C
      modes:
        - "off"
        - "cool"
        - "heat"
        - "dry"
        - "fan_only"
        - "auto"
      fan_modes:
        - "1"
        - "2"
        - "3"
        - "4"
        - "5"
        - "mute"
        - "turbo"
        - "auto"
      swing_modes:
        - "none"
        - "move_full"
        - "move_upper"
        - "move_lower"
        - "fix_top"
        - "fix_upper"
        - "fix_mid"
        - "fix_lower"
        - "fix_bottom"
      mode_command_topic: "cmnd/<amos_bedroom_device>/ACMode"
      mode_state_topic: "<amos_bedroom_device>/ACMode/get"
      mode_state_template: "{{ value if value != 'fan' else 'fan_only' }}"
      temperature_command_topic: "cmnd/<amos_bedroom_device>/TargetTemperature"
      temperature_state_topic: "<amos_bedroom_device>/TargetTemperature/get"
      current_temperature_topic: "<amos_bedroom_device>/CurrentTemperature/get"
      fan_mode_command_topic: "cmnd/<amos_bedroom_device>/FANMode"
      fan_mode_state_topic: "<amos_bedroom_device>/FANMode/get"
      swing_mode_command_topic: "cmnd/<amos_bedroom_device>/SwingV"
      swing_mode_state_topic: "<amos_bedroom_device>/SwingV/get"
      min_temp: 16
      max_temp: 30
      temp_step: 1
      availability_topic: "<amos_bedroom_device>/connected"
      payload_available: "online"
      payload_not_available: "offline"
```

Replace `<livingroom_device>`, `<kitchen_device>`, and `<amos_bedroom_device>` with the actual OpenBeken device names (e.g., `rtl87x0C0D1773DB`). These are typically based on the module's MAC address and visible on the OpenBeken web interface.

## Important Notes

1. **Protocol:** Della uses TCL, NOT TuyaMCU. If you see empty channels with TuyaMCU, switch to TCL driver.

2. **Temperature Unit:** Always set `temperature_unit: C` in Home Assistant config - the AC reports in Celsius.

3. **Mode Mapping:** The AC reports `fan` but Home Assistant expects `fan_only`. Use the template:
   ```yaml
   mode_state_template: "{{ value if value != 'fan' else 'fan_only' }}"
   ```

4. **Command Topic Format:** Commands use `cmnd/<device>/<property>` format, NOT `<device>/<property>/set`.

5. **MQTT Broker:** Ensure Mosquitto is running and Home Assistant MQTT integration is configured via UI (Settings → Devices & Services → MQTT).

## Troubleshooting

### No data from AC
- Verify TCL driver is active: check for "1 drivers active (TCL)" on index page
- Ensure AC unit is powered on
- Try restarting the OpenBeken device

### Commands not working
- Use `cmnd/` prefix for command topics
- Test via mosquitto_pub:
  ```bash
  mosquitto_pub -h <broker> -t "cmnd/<device>/TargetTemperature" -m "24"
  ```

### Temperature shows wrong unit
- Add `temperature_unit: C` to the climate config
- Restart Home Assistant

### TuyaMCU shows no channels
- This AC uses TCL protocol, not TuyaMCU
- Run: `stopDriver TuyaMCU` then `startDriver TCL`

## Quick Setup Commands

```bash
# Start TCL driver
curl "http://<device_ip>/cmd_tool?cmd=startDriver+TCL"

# Save to startup
curl "http://<device_ip>/startup_command?startup_cmd=1&data=startDriver%20TCL"

# Check status
curl "http://<device_ip>/index?state=1"

# Test MQTT command
mosquitto_pub -h <broker> -t "cmnd/<device>/ACMode" -m "cool"
```
