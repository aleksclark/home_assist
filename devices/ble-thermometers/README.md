# BLE Thermometers

Xiaomi Mijia Temperature/Humidity sensors (miaomiaoce.sensor_ht.t2) tracked via BLE.

## Sensors

Device credentials are stored in `devices.txt` with MAC addresses, BLE keys, and tokens.

| Name       | Location  | MAC               | Status    |
|------------|-----------|-------------------|-----------|
| 1          | Kitchen   | A4:C1:38:70:44:3C | Setup     |
| 2          | A&K BR    | A4:C1:38:8F:A0:C6 | Setup     |
| 3          | Amos      | A4:C1:38:A3:77:5D | Setup     |
| 4          | Red Room  | A4:C1:38:EE:77:6D | Setup     |

## Protocol

- **Firmware:** ATC MiThermometer (custom) or native Xiaomi BLE
- **ESPHome platform:** `atc_mithermometer`
- Sensors are read by the BLE scanner nodes; no per-sensor ESPHome config needed

## Adding a New Sensor

1. Obtain BLE key using Mi Home app or `telink_flasher`
2. Add credentials to `devices.txt`
3. Sensor will be auto-discovered by HA via BLE proxy nodes
