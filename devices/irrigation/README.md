# Irrigation Control

> **Status:** Planned

Two ESP32-based nodes for moisture sensing and automated irrigation.

## Nodes

### Indoor (`indoor/`)

- **Purpose:** Houseplant watering
- **Outputs:** 4 peristaltic pumps (one per plant/zone)
- **Sensors:** Capacitive soil moisture sensors
- **Board:** _TBD_

### Outdoor (`outdoor/`)

- **Purpose:** Garden drip irrigation
- **Outputs:** 4 solenoid valves controlling drip lines
- **Sensors:** Capacitive soil moisture sensors
- **Board:** _TBD_
- **Power:** Likely 24VAC solenoids with relay board

## Planned Features

- Per-zone moisture thresholds with hysteresis
- Manual override via Home Assistant
- Watering schedule fallback if HA is unreachable
- Moisture history logging
