# Outdoor Irrigation Controller — Wiring Diagram

## Modules

| Ref | Module | Qty |
|-----|--------|-----|
| U1 | ESP32-C3 CORE-ESP32 (32-pin) | 1 |
| U2 | GME12864 v3.22 OLED (SSD1306, I2C) | 1 |
| K1–K4 | CW-025 12V Relay Board (optocoupler, flyback) | 4 |
| S1–S4 | Capacitive Soil Moisture Sensor V2.0 | 4 |
| V1–V4 | 12V DC Solenoid Valve | 4 |
| PS1 | 12V DC Power Supply | 1 |
| DC1 | Buck Converter 12V → 5V (LM2596/MP1584) | 1 |
| R1–R2 | 4.7kΩ Resistor (I2C pull-up) | 2 |

## ESP32-C3 Pin Assignments

| U1 Pin | GPIO | Function | Connects To |
|--------|------|----------|-------------|
| IO00 | GPIO0 | ADC1_CH0 | S1 AOUT |
| IO01 | GPIO1 | ADC1_CH1 | S2 AOUT |
| IO03 | GPIO3 | ADC1_CH3 | S3 AOUT |
| IO04 | GPIO4 | ADC1_CH4 | S4 AOUT |
| IO05 | GPIO5 | I2C SDA | U2 SDA, R1 |
| IO06 | GPIO6 | I2C SCL | U2 SCL, R2 |
| IO07 | GPIO7 | Digital Out | K1 IN |
| IO10 | GPIO10 | Digital Out | K2 IN |
| IO20 | GPIO20 | Digital Out | K3 IN |
| IO21 | GPIO21 | Digital Out | K4 IN |
| 5V | — | Power In | DC1 OUT+ |
| 3V3 | — | 3.3V Out | S1–S4 VCC, U2 VCC, R1, R2 |
| GND | — | Ground | Common GND bus |

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

DC1 OUT+  ────── U1 5V
DC1 OUT−  ────── U1 GND
```

### I2C Bus (U1 → U2 OLED Display)

```
U1 IO05  ──┬── U2 SDA
            └── R1 (4.7kΩ) ── U1 3V3

U1 IO06  ──┬── U2 SCL
            └── R2 (4.7kΩ) ── U1 3V3

U1 3V3   ────── U2 VCC
U1 GND   ────── U2 GND
```

### Soil Moisture Sensors (U1 → S1–S4)

```
U1 IO00  ────── S1 AOUT
U1 IO01  ────── S2 AOUT
U1 IO03  ────── S3 AOUT
U1 IO04  ────── S4 AOUT

U1 3V3   ──┬── S1 VCC
            ├── S2 VCC
            ├── S3 VCC
            └── S4 VCC

U1 GND   ──┬── S1 GND
            ├── S2 GND
            ├── S3 GND
            └── S4 GND
```

### Relay Signal (U1 → K1–K4)

```
U1 IO07  ────── K1 IN
U1 IO10  ────── K2 IN
U1 IO20  ────── K3 IN
U1 IO21  ────── K4 IN

U1 GND   ──┬── K1 Signal GND
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
    PS1 (+12V) ─────────►│ DC1      │──► 5V ──► U1 5V
         │               │ 12V→5V  │
         │               └──────────┘
         │
         │    U1 ESP32-C3 CORE-ESP32
         │   ┌──────────────────────────┐
         │   │ IO00 ──── S1 AOUT        │
         │   │ IO01 ──── S2 AOUT        │   S1─S4: Capacitive
         │   │ IO03 ──── S3 AOUT        │   Soil Moisture
         │   │ IO04 ──── S4 AOUT        │   Sensors (3.3V)
         │   │                          │
         │   │ IO05 (SDA) ──┬── U2 SDA  │   U2: GME12864
         │   │ IO06 (SCL) ──┼── U2 SCL  │   OLED Display
         │   │              │  (4.7kΩ   │   (I2C, 0x3C)
         │   │              │  pull-ups) │
         │   │                          │
         │   │ IO07 ──── K1 IN          │
         │   │ IO10 ──── K2 IN          │   K1─K4: CW-025
         │   │ IO20 ──── K3 IN          │   12V Relay Boards
         │   │ IO21 ──── K4 IN          │
         │   └──────────────────────────┘
         │
         │   ┌──────────────────────────────────┐
         ├──►│ K1 COM←12V  NO──►V1(+)  V1(−)──►GND │
         ├──►│ K2 COM←12V  NO──►V2(+)  V2(−)──►GND │
         ├──►│ K3 COM←12V  NO──►V3(+)  V3(−)──►GND │
         └──►│ K4 COM←12V  NO──►V4(+)  V4(−)──►GND │
             └──────────────────────────────────┘
                     V1─V4: 12V DC Solenoid Valves
```

## Notes

- **Relay logic:** Non-inverted — GPIO HIGH = relay energized = valve open
- **I2C address:** U2 OLED at 0x3C
- **Strapping pins avoided:** GPIO2, GPIO8, GPIO9 not used (boot conflicts)
- **ADC:** All sensors on ADC1 channels (ADC2 unreliable with WiFi active)
- **Sensor cable:** Use shielded 3-conductor cable for runs > 1m
