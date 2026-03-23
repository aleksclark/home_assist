# Home Assistant

Docker-based Home Assistant instance running on `192.168.0.3`.

## Integrations

| Integration        | Type         | Notes                                    |
|--------------------|--------------|------------------------------------------|
| ESPHome            | Native API   | BLE scanners, Della minisplits           |
| MQTT (Mosquitto)   | MQTT         | Della mini splits via OpenBeken          |
| Bluetooth Proxy    | BLE          | Via ESP32-C3 scanner nodes               |

## HVAC Zone Control

### Architecture

The house runs **3 Della minisplits** (MQTT/OpenBeken) covering the kitchen,
amos bedroom, and livingroom.  The ecobee central system is **not used** for
automated control.

Each minisplit automation evaluates current room temperature against the
active schedule band and decides heat/cool/off:

```
current_temp < heat_min  →  heat mode, setpoint = heat_min
current_temp > cool_max  →  cool mode, setpoint = cool_max
within band              →  off
```

The livingroom is **not occupied** and instead thermally tracks the average
of kitchen + amos room temperatures, staying within ±5°F.

### Zone Map

| Zone           | Area   | HVAC Device           | Temp Source                           |
|----------------|--------|-----------------------|---------------------------------------|
| Kitchen        | 504 ft²| Della 18k BTU (MQTT)  | `climate.kitchen_ac` current_temp + `sensor.kitchen_temp_temperature` (BLE) |
| Amos Bedroom   | 210 ft²| Della 9k BTU (MQTT)   | `climate.amos_bedroom_ac` current_temp + `sensor.atc_48af_temperature` (BLE) |
| Livingroom     | 192 ft²| Della 12k BTU (MQTT)  | `climate.livingroom_ac` current_temp (thermal tracking only) |
| Hallway        | 53 ft² | Matter thermostat     | `climate.smart_thermostat` current_temp (heat_cool dual setpoint) |

### Schedule (Kitchen, Amos, Hallway)

All 4 periods apply identically to kitchen, amos bedroom, and hallway.
Dellas use heat/cool/off logic; hallway uses `heat_cool` mode with dual setpoints.

| Period          | Time            | Heat Min (°F) | Cool Max (°F) |
|-----------------|-----------------|---------------|----------------|
| Early Morning   | 05:30 – 06:30   | 68            | 75             |
| Morning         | 06:30 – 08:30   | 63            | 70             |
| Daytime         | 08:30 – 21:00   | 68            | 75             |
| Overnight       | 21:00 – 05:30   | 62            | 68             |

### Livingroom Thermal Tracking

The livingroom has no direct occupancy schedule.  It is kept within a
configurable band (default ±5°F) of the average of:
- `sensor.kitchen_temp_temperature` (kitchen BLE thermometer)
- `sensor.atc_48af_temperature` (amos BLE thermometer)

### Automations (5 total)

| ID | Triggers | Action |
|----|----------|--------|
| `hvac_kitchen_schedule` | Period boundaries + every 10 min | Kitchen AC → heat/cool/off based on band |
| `hvac_amos_schedule` | Period boundaries + every 10 min | Amos AC → heat/cool/off based on band |
| `hvac_hallway_schedule` | Period boundaries + every 10 min | Hallway → heat_cool with target_temp_low/high from band |
| `hvac_livingroom_thermal_tracking` | Every 10 min + sensor changes | Livingroom AC → track avg(kitchen, amos) ±5°F |
| `hvac_master_disable` | Master toggle off | All 4 units off |

### Input Helpers

| Entity | Default | Purpose |
|--------|---------|---------|
| `input_boolean.hvac_master_enable` | on | Kill switch for all HVAC automations |
| `input_number.hvac_early_morning_heat_f` | 68°F | 05:30–06:30 heat floor |
| `input_number.hvac_early_morning_cool_f` | 75°F | 05:30–06:30 cool ceiling |
| `input_number.hvac_morning_heat_f` | 63°F | 06:30–08:30 heat floor |
| `input_number.hvac_morning_cool_f` | 70°F | 06:30–08:30 cool ceiling |
| `input_number.hvac_daytime_heat_f` | 68°F | 08:30–21:00 heat floor |
| `input_number.hvac_daytime_cool_f` | 75°F | 08:30–21:00 cool ceiling |
| `input_number.hvac_overnight_heat_f` | 62°F | 21:00–05:30 heat floor |
| `input_number.hvac_overnight_cool_f` | 68°F | 21:00–05:30 cool ceiling |
| `input_number.hvac_livingroom_thermal_band_f` | 5°F | Livingroom max deviation from avg |

### Key Design Decisions

- **No ecobee automation.**  Central system is not used for automated control.
- **Fahrenheit throughout.**  HA unit system is °F.
- **Automation-driven heat/cool selection.**  Each 10-minute cycle reads current
  temp and compares against the active band — below min = heat, above max = cool,
  within band = off.  No reliance on the Della's auto mode.
- **Livingroom thermal tracking** uses BLE sensors from adjacent rooms as reference
  since the livingroom has no occupancy schedule of its own.
- **Season toggles removed.**  The heat/cool decision is now fully automatic based
  on where current temp falls relative to the band.
- **Input helpers are set via API** (not persistent across HA restart).  Add
  `input_helpers.yaml` to `configuration.yaml` for persistence.

## Files

| File | Purpose |
|------|---------|
| `README.md` | This file |
| `zones.yaml` | House zone topology, thermal coupling, device map, schedules |
| `mqtt.yaml` | MQTT climate entity definitions for 3 Della minisplits |
| `automations_hvac.yaml` | Reference copy of deployed automations (YAML) |
| `input_helpers.yaml` | Reference copy of input_boolean / input_number definitions |
