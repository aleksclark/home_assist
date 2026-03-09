# BLE Scanners

ESP32-C3 devices running ESPHome as Bluetooth LE proxy nodes for Home Assistant.

## Hardware

- **Board:** ESP32-C3-DevKitM-1
- **Framework:** ESP-IDF
- **Function:** Bluetooth proxy + optional BLE sensor reading

## Deployed Nodes

| Location    | Config File                       | WiFi Workaround |
|-------------|-----------------------------------|-----------------|
| Office      | `ble-scanner-office.yaml`         | Yes             |
| Kitchen     | `ble-scanner-kitchen.yaml`        | Yes             |
| Livingroom  | `ble-scanner-livingroom.yaml`     | Yes             |
| Bedroom     | `ble-scanner-bedroom.yaml`        | No              |
| Bathroom    | `ble-scanner-bathroom.yaml`       | No              |
| Carport     | `ble-scanner-carport.yaml`        | No              |

## Commands

```bash
esphome compile devices/ble-scanners/ble-scanner-<location>.yaml
esphome upload devices/ble-scanners/ble-scanner-<location>.yaml
esphome logs devices/ble-scanners/ble-scanner-<location>.yaml
```

## Known Issues

- `ble-scanner-livingroom.yaml` has a typo: `name: ble-scanner-living-toom`
- Some ESP32-C3 modules need the 802.11b/g WiFi workaround for stability
