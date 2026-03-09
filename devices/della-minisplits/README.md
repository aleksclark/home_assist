# Della Mini Split AC Units

Della mini split units flashed with OpenBeken firmware, controlled via TCL protocol over MQTT.

## Overview

- **Chip:** RTL87x0C (Realtek)
- **Firmware:** OpenBeken
- **Protocol:** TCL (NOT TuyaMCU)
- **Integration:** MQTT → Home Assistant

See [SETUP.md](SETUP.md) for flashing and configuration details.

## Quick Reference

```bash
# Start TCL driver on a unit
curl "http://<device_ip>/cmd_tool?cmd=startDriver+TCL"

# Check status
curl "http://<device_ip>/index?state=1"
```
