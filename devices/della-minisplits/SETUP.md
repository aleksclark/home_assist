# Della Mini Split AC - OpenBeken Setup Guide

## Overview

Della mini split AC units with built-in WiFi use the **TCL protocol**, NOT TuyaMCU. The WiFi module is typically an RTL87x0C-based chip that can be flashed with OpenBeken firmware.

## Hardware

- **Chip:** RTL87x0C (Realtek)
- **Protocol:** TCL (serial communication with AC MCU)
- **Default Device Name:** `rtl87x0C0D1773DB` (based on MAC address)

## OpenBeken Configuration

### 1. Flash OpenBeken

Flash the module with OpenBeken firmware. The module should be accessible at its IP address after connecting to WiFi.

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
    - name: "Della Mini Split"
      unique_id: "della_mini_split_ac"
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
      mode_command_topic: "cmnd/<device>/ACMode"
      mode_state_topic: "<device>/ACMode/get"
      mode_state_template: "{{ value if value != 'fan' else 'fan_only' }}"
      temperature_command_topic: "cmnd/<device>/TargetTemperature"
      temperature_state_topic: "<device>/TargetTemperature/get"
      current_temperature_topic: "<device>/CurrentTemperature/get"
      fan_mode_command_topic: "cmnd/<device>/FANMode"
      fan_mode_state_topic: "<device>/FANMode/get"
      swing_mode_command_topic: "cmnd/<device>/SwingV"
      swing_mode_state_topic: "<device>/SwingV/get"
      min_temp: 16
      max_temp: 30
      temp_step: 1
      availability_topic: "<device>/connected"
      payload_available: "online"
      payload_not_available: "offline"
```

Replace `<device>` with the actual device name (e.g., `rtl87x0C0D1773DB`).

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
