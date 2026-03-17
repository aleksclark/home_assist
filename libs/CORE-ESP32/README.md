# CORE-ESP32-C3

LuatOS/AirM2M CORE-ESP32-C3 development board — a compact ESP32-C3 core board with USB-C, onboard LEDs, and castellated/stamp-hole edges.

## Specifications

| Parameter          | Value                                      |
|--------------------|--------------------------------------------|
| MCU                | ESP32-C3 (RISC-V 32-bit, single core)     |
| Clock              | Up to 160 MHz                              |
| Flash              | 4 MB (external SPI, supports up to 16 MB)  |
| SRAM               | 400 KB (16 KB cache)                       |
| RTC SRAM           | 8 KB                                       |
| ROM                | 384 KB                                     |
| WiFi               | 802.11 b/g/n (2.4 GHz)                    |
| Bluetooth          | BLE 5.0                                    |
| USB                | Type-C (power + serial)                    |
| Antenna            | 2.4 GHz PCB onboard                       |
| Board dimensions   | 21 mm × 51 mm                              |
| Pin count          | 32 (2×16, 2.54 mm pitch)                  |
| Operating voltage  | 3.3 V (LDO from 5 V USB)                  |

## Pinout

Pins 1–16 run down the left side, 17–32 down the right side. Pin 1 is top-left (nearest the USB connector).

```
                     USB-C
                  ┌───┴───┐
         GND  1  ─┤       ├─  17  VBUS
        VBUS  2  ─┤       ├─  18  PWB
  GPIO9/BOOT  3  ─┤       ├─  19  GND
   GPIO8/PWM  4  ─┤       ├─  20  +3V3
 GPIO4/SDA    5  ─┤       ├─  21  CHIP_EN
 GPIO5/SCL    6  ─┤       ├─  22  NC
        +3V3  7  ─┤       ├─  23  SPIWP/GPIO13
         GND  8  ─┤       ├─  24  U0TXD
 VDD_SPI/IO11 9  ─┤       ├─  25  U0RXD
       GPIO7 10  ─┤       ├─  26  GND
       GPIO6 11  ─┤       ├─  27  GPIO19/USB_D+
      GPIO10 12  ─┤       ├─  28  GPIO18/USB_D-
       GPIO3 13  ─┤       ├─  29  SPIHD/GPIO12
       GPIO2 14  ─┤       ├─  30  GPIO01/U1RXD
        +3V3 15  ─┤       ├─  31  GPIO00/U1TXD
         GND 16  ─┤       ├─  32  GND
                  └───────┘
```

### Pin Functions

| Pin | Name           | GPIO   | Alternate Functions              |
|-----|----------------|--------|----------------------------------|
| 1   | GND            | —      | Ground                           |
| 2   | VBUS           | —      | 5 V (USB VBUS)                   |
| 3   | GPIO9/BOOT     | GPIO9  | Boot mode select                 |
| 4   | GPIO8/PWM      | GPIO8  | PWM                              |
| 5   | GPIO4/SDA      | GPIO4  | I2C_SDA, ADC1                    |
| 6   | GPIO5/SCL      | GPIO5  | I2C_SCL, ADC2                    |
| 7   | +3V3           | —      | 3.3 V power output               |
| 8   | GND            | —      | Ground                           |
| 9   | VDD_SPI/GPIO11 | GPIO11 | VDD_SPI (can unlock as GPIO)     |
| 10  | GPIO7          | GPIO7  | —                                |
| 11  | GPIO6          | GPIO6  | —                                |
| 12  | GPIO10         | GPIO10 | —                                |
| 13  | GPIO3          | GPIO3  | —                                |
| 14  | GPIO2          | GPIO2  | —                                |
| 15  | +3V3           | —      | 3.3 V power output               |
| 16  | GND            | —      | Ground                           |
| 17  | VBUS           | —      | 5 V (USB VBUS)                   |
| 18  | PWB            | —      | 3.3 V power control              |
| 19  | GND            | —      | Ground                           |
| 20  | +3V3           | —      | 3.3 V power output               |
| 21  | CHIP_EN        | —      | Chip enable (active high reset)  |
| 22  | NC             | —      | No connect                       |
| 23  | SPIWP/GPIO13   | GPIO13 | SPIWP                            |
| 24  | U0TXD          | GPIO21 | UART0 TX                         |
| 25  | U0RXD          | GPIO20 | UART0 RX                         |
| 26  | GND            | —      | Ground                           |
| 27  | GPIO19/USB_D+  | GPIO19 | USB D+                           |
| 28  | GPIO18/USB_D-  | GPIO18 | USB D−                           |
| 29  | SPIHD/GPIO12   | GPIO12 | SPIHD                            |
| 30  | GPIO01/U1RXD   | GPIO1  | UART1 RX                         |
| 31  | GPIO00/U1TXD   | GPIO0  | UART1 TX                         |
| 32  | GND            | —      | Ground                           |

### Peripheral Mapping

| Peripheral | Pins                          |
|------------|-------------------------------|
| UART0      | TX = pin 24 (GPIO21), RX = pin 25 (GPIO20) |
| UART1      | TX = pin 31 (GPIO0), RX = pin 30 (GPIO1)   |
| I2C        | SDA = pin 5 (GPIO4), SCL = pin 6 (GPIO5)   |
| USB        | D+ = pin 27 (GPIO19), D− = pin 28 (GPIO18) |
| ADC        | GPIO4 (pin 5), GPIO5 (pin 6)  |
| PWM        | Any GPIO (4 channels max)     |

## Strapping / Boot Pins

| Pin    | Function                                            |
|--------|-----------------------------------------------------|
| GPIO8  | Avoid external pull-down (used during programming)  |
| GPIO9  | Low at reset enters download mode (BOOT button)     |
| GPIO11 | Defaults to VDD_SPI; unlockable via eFuse           |

## Power

Three ways to power the board:

1. **USB-C** — 5 V via the onboard connector (recommended for development)
2. **VBUS pins** (2, 17) — External 5 V supply, regulated to 3.3 V by onboard LDO
3. **+3V3 pins** (7, 15, 20) — Direct 3.3 V supply, bypasses LDO

**PWB** (pin 18) controls the 3.3 V regulator output and can be used to power-gate the board from an external circuit.

## Onboard Components

- 2 × LEDs: D4 (GPIO12), D5 (GPIO13) — active high
- 2 × Buttons: BOOT (GPIO9), RST (reset)
- USB-to-UART bridge: CH343 (classic revision) or native USB (newer revision)
- LDO: 5 V → 3.3 V
- SPI flash: 4 MB

## KiCad Library

Symbol and footprint are provided in this directory for KiCad 9:

```
libs/CORE-ESP32/
├── CORE-ESP32-C3.kicad_sym           # schematic symbol
└── CORE-ESP32-C3.pretty/
    └── CORE-ESP32-C3.kicad_mod       # PCB footprint (21×51 mm, 2×16 THT)
```

To add to a KiCad project:

1. **Symbol library** — Preferences → Manage Symbol Libraries → Project Specific → add `CORE-ESP32-C3.kicad_sym`
2. **Footprint library** — Preferences → Manage Footprint Libraries → Project Specific → add `CORE-ESP32-C3.pretty/`

## References

- [LuatOS CORE-ESP32-C3 wiki](https://wiki.luatos.org/chips/esp32c3/board.html)
- [LuatOS BOM / pin table](https://wiki.luatos.org/_static/bom/esp32c3.html)
- [ESP32-C3 datasheet (Espressif)](https://www.espressif.com/sites/default/files/documentation/esp32-c3_datasheet_en.pdf)
