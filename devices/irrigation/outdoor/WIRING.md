# Outdoor Irrigation Controller — Wiring Diagram
OVERRIDE:

S3 AOUT -> IO12
## Modules

| Ref | Module | Qty |
|-----|--------|-----|
| U1 | CORE-ESP32-C3 (LuatOS/AirM2M, 32-pin) | 1 |
| U2 | GME12864 v3.22 OLED (SSD1306, I2C) | 1 |
| K1–K4 | CW-025 12V Relay Board (optocoupler, flyback) | 4 |
| S1–S4 | Capacitive Soil Moisture Sensor V2.0 | 4 |
| V1–V4 | 12V DC Solenoid Valve | 4 |
| PS1 | 12V DC Power Supply | 1 |
| DC1 | Buck Converter 12V → 5V (LM2596/MP1584) | 1 |
| R1–R2 | 4.7kΩ Resistor (I2C pull-up) | 2 |

## CORE-ESP32-C3 Pin Assignments

| U1 Pin # | Board Label | GPIO | Function | Connects To |
|----------|-------------|------|----------|-------------|
| 2 | IO00 | GPIO0 | ADC_0 | S1 AOUT |
| 3 | IO01 | GPIO1 | ADC_1 | S2 AOUT |
| 19 | IO02 | GPIO2 | ADC_2 | S3 AOUT |
| 20 | IO03 | GPIO3 | ADC_3 | S4 AOUT |
| 28 | IO04 | GPIO4 | I2C SDA | U2 SDA, R1 |
| 27 | IO05 | GPIO5 | I2C SCL | U2 SCL, R2 |
| 23 | IO07 | GPIO7 | Digital Out | K1 IN |
| 21 | IO10 | GPIO10 | Digital Out | K2 IN |
| 22 | IO06 | GPIO6 | Digital Out | K3 IN |
| 8 | IO13 | GPIO13 | Digital Out | K4 IN |
| 16 | 5V | — | Power In | DC1 OUT+ |
| 13 | 3.3V | — | 3.3V Out | S1–S4 VCC, U2 VCC, R1, R2 |
| 1 | GND | — | Ground | Common GND bus |

### Pins NOT used (reserved / special)

| Pin # | Label | GPIO | Reason |
|-------|-------|------|--------|
| 4 | IO12 | GPIO12 | SPI flash / LED D4 |
| 5 | IO18 | GPIO18 | USB D− |
| 6 | IO19 | GPIO19 | USB D+ |
| 9 | U0_TX | GPIO21 | UART0 TX (spare) |
| 10 | IO13 | GPIO13 | SPI flash (unavailable as GPIO) |
| 29 | IO08 | GPIO8 | Strapping pin |
| 30 | BOOT | GPIO9 | Boot mode select |
| 24 | PB_11 | GPIO11 | VDD_SPI (locked) |

## Connection List

### Power

```
PS1 (+12V) ──┬── K1 VCC
              ├── K2 VCC
              ├── K3 VCC
              ├── K4 VCC
              ├── V1–V4 (via relay COM, see below)
              └── DC1 IN+

PS1 (GND)  ──┬── K1 GND
              ├── K2 GND
              ├── K3 GND
              ├── K4 GND
              ├── V1–V4 (−) terminal
              ├── DC1 IN−
              └── Common GND bus

DC1 OUT+  ────── U1 pin 16 (5V)
DC1 OUT−  ────── U1 pin 1 (GND)
```

### I2C Bus (U1 → U2 OLED Display)

```
U1 pin 28 (IO04/SDA) ──┬── U2 SDA
                        └── R1 (4.7kΩ) ── U1 pin 13 (3.3V)

U1 pin 27 (IO05/SCL) ──┬── U2 SCL
                        └── R2 (4.7kΩ) ── U1 pin 13 (3.3V)

U1 pin 13 (3.3V) ────── U2 VCC
U1 pin 1  (GND)  ────── U2 GND
```

### Soil Moisture Sensors (U1 → S1–S4)

```
U1 pin 2  (IO00) ────── S1 AOUT
U1 pin 3  (IO01) ────── S2 AOUT
U1 pin 19 (IO02) ────── S3 AOUT
U1 pin 20 (IO03) ────── S4 AOUT

U1 pin 13 (3.3V) ──┬── S1 VCC
                    ├── S2 VCC
                    ├── S3 VCC
                    └── S4 VCC

U1 pin 1  (GND)  ──┬── S1 GND
                    ├── S2 GND
                    ├── S3 GND
                    └── S4 GND
```

### Relay Signal (U1 → K1–K4)

```
U1 pin 23 (IO07)  ────── K1 IN
U1 pin 21 (IO10)  ────── K2 IN
U1 pin 22 (IO06)  ────── K3 IN
U1 pin 8  (U0_RX) ────── K4 IN

U1 pin 1  (GND)   ──┬── K1 Signal GND
                     ├── K2 Signal GND
                     ├── K3 Signal GND
                     └── K4 Signal GND
```

### Relay → Solenoid Valves (K1–K4 → V1–V4)

```
K1 COM  ────── PS1 (+12V)        K1 NO  ────── V1 (+)
K2 COM  ────── PS1 (+12V)        K2 NO  ────── V2 (+)
K3 COM  ────── PS1 (+12V)        K3 NO  ────── V3 (+)
K4 COM  ────── PS1 (+12V)        K4 NO  ────── V4 (+)

V1 (−)  ────── PS1 GND
V2 (−)  ────── PS1 GND
V3 (−)  ────── PS1 GND
V4 (−)  ────── PS1 GND
```

## Block Diagram

```
                         ┌──────────┐
    PS1 (+12V) ─────────►│ DC1      │──► 5V ──► U1 pin 16
         │               │ 12V→5V  │
         │               └──────────┘
         │
         │    U1 CORE-ESP32-C3
         │   ┌──────────────────────────────────┐
         │   │ pin 2  IO00 ──── S1 AOUT         │
         │   │ pin 3  IO01 ──── S2 AOUT         │  S1─S4: Capacitive
         │   │ pin 19 IO02 ──── S3 AOUT         │  Soil Moisture
         │   │ pin 20 IO03 ──── S4 AOUT         │  Sensors (3.3V)
         │   │                                  │
         │   │ pin 28 IO04 (SDA) ──┬── U2 SDA   │  U2: GME12864
         │   │ pin 27 IO05 (SCL) ──┼── U2 SCL   │  OLED Display
         │   │                     │  (4.7kΩ    │  (I2C, 0x3C)
         │   │                     │  pullups)  │
         │   │                                  │
         │   │ pin 23 IO07  ──── K1 IN          │
         │   │ pin 21 IO10  ──── K2 IN          │  K1─K4: CW-025
         │   │ pin 22 IO06  ──── K3 IN          │  12V Relay Boards
         │   │ pin 8  U0_RX ──── K4 IN          │
         │   └──────────────────────────────────┘
         │
         │   ┌──────────────────────────────────────┐
         ├──►│ K1 COM←12V  NO──►V1(+)  V1(−)──►GND │
         ├──►│ K2 COM←12V  NO──►V2(+)  V2(−)──►GND │
         ├──►│ K3 COM←12V  NO──►V3(+)  V3(−)──►GND │
         └──►│ K4 COM←12V  NO──►V4(+)  V4(−)──►GND │
             └──────────────────────────────────────┘
                      V1─V4: 12V DC Solenoid Valves
```

## Notes

- **Board:** LuatOS CORE-ESP32-C3 (21×51mm, 2×16 pins, 2.54mm pitch)
- **I2C:** Uses board's designated I2C pins — SDA=GPIO4 (pin 28), SCL=GPIO5 (pin 27)
- **ADC:** All 4 sensors on ADC1 channels 0–3 (GPIO0–3), reliable with WiFi active
- **Relay logic:** Non-inverted — GPIO HIGH = relay energized = valve open
- **GPIO20 (U0_RX):** Repurposed from UART0 RX for relay K4; serial console still available via USB
- **GPIO13:** Reserved by ESP-IDF for SPI flash interface, cannot be used as GPIO
- **Strapping pins:** GPIO2 used for ADC only (safe), GPIO8/GPIO9 not used
- **Sensor cable:** Use shielded 3-conductor cable for runs > 1m
