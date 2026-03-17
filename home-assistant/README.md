# Home Assistant

Docker-based Home Assistant instance running on `192.168.0.3`.

## Integrations

| Integration        | Type         | Notes                                    |
|--------------------|--------------|------------------------------------------|
| ESPHome            | Native API   | BLE scanners, future ESP devices         |
| MQTT (Mosquitto)   | MQTT         | Della mini splits via OpenBeken          |
| Ecobee             | Cloud/Local  | ECB701 Smart Thermostat Essential        |
| Bluetooth Proxy    | BLE          | Via ESP32-C3 scanner nodes               |

## HVAC Zone Control

### Architecture

The house has a **hybrid HVAC system**: 3 Della minisplits (MQTT/OpenBeken)
covering individual rooms, and 1 central AC system (ecobee ECB701) serving the
hallway wing.  An AI-agent-friendly zone topology file (`zones.yaml`) describes
the spatial layout, thermal coupling between rooms, device capabilities, and
occupancy schedules.

```
                    ┌─────────────┐
                    │  zones.yaml │  (topology + schedule)
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
     ┌────────────┐ ┌───────────┐ ┌──────────────┐
     │ Minisplits │ │  Ecobee   │ │   Conflict   │
     │ (3 zones)  │ │ (central) │ │    Guards    │
     └─────┬──────┘ └─────┬─────┘ └──────┬───────┘
           │              │              │
           ▼              ▼              ▼
     ┌──────────────────────────────────────────┐
     │         HA Automations (13 total)        │
     │    driven by input_boolean + input_number │
     │              helper entities              │
     └──────────────────────────────────────────┘
```

### Zone Map

| Zone           | Area   | HVAC Device           | Sensor                          |
|----------------|--------|-----------------------|---------------------------------|
| Kitchen        | 504 ft²| Della 18k BTU (MQTT)  | `sensor.kitchen_temp_temperature` |
| Livingroom     | 192 ft²| Della 12k BTU (MQTT)  | none                            |
| Amos Bedroom   | 210 ft²| Della 9k BTU (MQTT)   | none (device offline)           |
| Hallway        | 53 ft² | Central 30k BTU       | `sensor.my_ecobee_current_temperature_2` |
| A&K Bedroom    | 210 ft²| Central (via ducts)   | `sensor.atc_a0c6_temperature`   |
| Girls Bedroom  | 550 ft²| Central (via ducts)   | none                            |
| Girls Bathroom | 120 ft²| Central (via ducts)   | none                            |
| Green Room     | 120 ft²| none (thermal bridge) | none                            |

### Automations

Deployed via the HA `save_automation` API.  13 automations, all UI-managed:

| ID | Trigger | Action |
|----|---------|--------|
| `hvac_kitchen_occupied_start` | 05:00 | Kitchen AC → occupied target |
| `hvac_kitchen_unoccupied_start` | 21:00 | Kitchen AC → unoccupied target |
| `hvac_livingroom_occupied_start` | 05:00 | Livingroom AC → occupied target |
| `hvac_livingroom_unoccupied_start` | 21:00 | Livingroom AC → unoccupied target |
| `hvac_amos_unoccupied_start` | 05:00 | Amos AC → unoccupied target |
| `hvac_amos_occupied_start` | 15:00 | Amos AC → occupied target |
| `hvac_amos_sleep_start` | 20:00 | Amos AC → sleep target |
| `hvac_central_occupied_start` | 05:00 | Ecobee → occupied heat_cool band |
| `hvac_central_sleep_start` | 21:00 | Ecobee → sleep heat_cool band |
| `hvac_conflict_guard_cool` | Central heats | Turn off minisplits |
| `hvac_conflict_guard_heat` | Central cools | Turn off heating minisplits |
| `hvac_conflict_guard_restore` | Central idle 5m | Restore minisplits to seasonal mode |
| `hvac_master_disable` | Master toggle off | All units off |

### Input Helpers

Adjustable from the HA dashboard.  Created via `set_state` API (not persistent
across HA restart — add `input_helpers.yaml` to `configuration.yaml` for
persistence).

| Entity | Default | Purpose |
|--------|---------|---------|
| `input_boolean.hvac_master_enable` | on | Kill switch for all HVAC automations |
| `input_boolean.hvac_season_cool` | on | Selects cooling mode for minisplits |
| `input_boolean.hvac_season_heat` | off | Selects heating mode for minisplits |
| `input_number.hvac_occupied_heat_f` | 67°F | Heat target during occupied hours |
| `input_number.hvac_occupied_cool_f` | 75°F | Cool target during occupied hours |
| `input_number.hvac_sleep_heat_f` | 60°F | Heat target during sleep hours |
| `input_number.hvac_sleep_cool_f` | 68°F | Cool target during sleep hours |
| `input_number.hvac_unoccupied_heat_f` | 62°F | Heat target when unoccupied |
| `input_number.hvac_unoccupied_cool_f` | 82°F | Cool target when unoccupied |

### Key Design Decisions

- **Fahrenheit throughout.**  HA unit system is °F.  The Della MQTT entities
  report min/max in °F (61–86) after HA conversion.  Ecobee is native °F.  No
  Celsius conversion needed in automations.
- **Ecobee uses `heat_cool` mode** with `target_temp_low` / `target_temp_high`
  rather than switching between heat and cool, since the central system serves
  multiple rooms and conditions can vary.
- **Conflict guards** prevent minisplits and central AC from fighting each
  other through the green room thermal bridge (high coupling to both kitchen
  and hallway).
- **Season is manual** (`input_boolean.hvac_season_cool`) rather than derived
  from `sensor.season`, giving control during shoulder seasons.
- **Livingroom mirrors kitchen schedule** since it has no explicit occupancy
  data and shares a wall with the kitchen.

## Performance Review (March 10–11, 2026)

First 36 hours of data after automation deployment.  All temperatures are
hourly means from the HA recorder.

### Kitchen (Della 18k BTU, 504 ft²)

Target: 75°F cool (occupied 05:00–21:00), 82°F cool (unoccupied).

| Period | Temp Range | vs. Target | Notes |
|--------|-----------|------------|-------|
| Occupied (daytime) | 65–73°F | 2–10°F under | Never exceeded ceiling |
| Occupied (early AM dip) | 65.6–67.4°F | 8–9°F under | Overcooling before dawn |
| Unoccupied (night) | 66–72°F | 10–16°F under | Well within ceiling |

**Grade: A** — Always at or below target.  Arguably overcools, especially
overnight.  The 18k BTU unit is oversized for this room.

### A&K Bedroom (Central AC, 210 ft²)

Target: 75°F cool (occupied 05:00–21:00), 68°F cool (sleep 21:00–05:00).

| Period | Temp Range | vs. Target | Notes |
|--------|-----------|------------|-------|
| Occupied | 69–70°F | 5°F under | Comfortable |
| Sleep | 71–72°F | **+3°F over** | Central can't pull room to 68°F |
| Unoccupied | 68–70°F | 12°F under | Fine |

**Grade: B-** — Occupied hours are good.  Sleep target of 68°F is consistently
missed by ~3°F.

### Hallway / Ecobee (Central 30k BTU, 53 ft²)

Target: 67–75°F heat_cool (occupied), 60–68°F heat_cool (sleep).

| Period | Temp Range | vs. Target | Notes |
|--------|-----------|------------|-------|
| Occupied | 67–68°F | In band | Hugs the heat floor |
| Sleep (first 3h) | 69–70°F | **+1.5°F over** | Residual heat from adjacent rooms |
| Sleep (after midnight) | 67–68°F | In band | Catches up |

**Grade: B** — Slight overshoot at sleep transition, resolves within a few
hours.

### Amos Bedroom (Della 9k BTU, 210 ft²)

**No data.**  The minisplit (`climate.amos_bedroom_ac`) and all OpenBeken
sensors show `unavailable`.  The BLE thermometer for this room
(`A4:C1:38:A3:77:5D`) does not have a corresponding HA entity.

**Grade: Incomplete.**

### Scorecard

| Zone | Occupied | Sleep | Unoccupied | Overall |
|------|----------|-------|------------|---------|
| Kitchen | A | — | A | **A** |
| A&K Bedroom | A | C | A | **B-** |
| Hallway | A | B | — | **B** |
| Amos Bedroom | ? | ? | ? | **?** |

## Open Issues

1. **Amos minisplit offline.**  The OpenBeken device is unreachable.  Needs
   hardware-level troubleshooting (power, WiFi, firmware).  The Amos BLE
   thermometer is also not registered in HA.

2. **A&K bedroom sleep target miss.**  Central AC cannot cool this room to
   68°F.  Options:
   - Pre-cool: shift sleep setpoint 1 hour earlier (20:00 instead of 21:00)
   - Add a dedicated minisplit
   - Rebalance duct dampers (ensure A&K duct is fully open)

3. **Kitchen overcooling.**  The 18k BTU unit drives the 504 ft² room well
   below the 75°F target.  Consider per-zone target overrides or lowering fan
   speed during overnight hours.

4. **Input helpers not persistent.**  Created via `set_state` API — will be
   lost on HA restart.  To persist, add the contents of `input_helpers.yaml`
   to `configuration.yaml` and restart HA.

5. **No sensors in livingroom, girls bedroom, girls bathroom, green room.**
   Impossible to verify HVAC performance in these zones.  Adding BLE
   thermometers would close the feedback loop.

6. **No per-zone target overrides.**  All rooms share the same global setpoints.
   The kitchen doesn't need the same target as a 210 ft² bedroom.

## Files

| File | Purpose |
|------|---------|
| `README.md` | This file |
| `zones.yaml` | House zone topology, thermal coupling, device map, schedules |
| `mqtt.yaml` | MQTT climate entity definitions for 3 Della minisplits |
| `automations_hvac.yaml` | Reference copy of deployed automations (YAML) |
| `input_helpers.yaml` | Reference copy of input_boolean / input_number definitions |
