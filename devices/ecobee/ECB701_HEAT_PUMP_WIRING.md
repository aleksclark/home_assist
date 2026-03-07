# ECB701 Wiring — Heat Pump (Air or Geothermal) with Auxiliary Heat

> Based on the official ecobee wiring diagram for heat pump systems with aux heat.

---

## System Components

| Component | Role |
|-----------|------|
| **Heat pump** (outdoor unit) | Primary heating and cooling via refrigerant cycle |
| **Air handler** (indoor unit) | Blower, electric heat strips (aux/emergency), evaporator coil |
| **ecobee ECB701** | Thermostat controlling both units |

---

## ecobee Backplate Terminals

The ECB701 backplate has two columns of terminals:

### Left Column (Cooling / Heat Pump)

| Terminal | Function |
|----------|----------|
| **Rc**   | 24 V power — R wire lands here |
| **G**    | Indoor blower fan |
| **Y1**   | Stage 1 compressor (heat pump) |
| **Y2**   | Stage 2 compressor (if applicable) |
| **O/B**  | Heat pump reversing valve |

### Right Column (Heating / Air Handler)

| Terminal | Function |
|----------|----------|
| **Rh**   | Internally jumpered to Rc — leave empty unless dual-transformer system |
| **C**    | 24 V common (return to transformer) |
| **W1**   | Stage 1 auxiliary heat (electric heat strips) |
| **W2**   | Stage 2 auxiliary heat (if applicable) |
| **PEK**  | Power Extender Kit connector |

---

## Wire Routing

### Heat Pump (Outdoor Unit) Terminal Strip

```
O/B    W2    Y1    Y2    R     C
 │      │     │     │     │     │
 │      │     │     │     │     │
```

Wires from this strip run to the ecobee:

| Heat Pump Terminal | ecobee Terminal |
|--------------------|-----------------|
| O/B                | O/B             |
| Y1                 | Y1              |
| Y2                 | Y2 (if present) |
| R                  | Rc              |
| C                  | C               |

### Air Handler (Indoor Unit) Terminal Strip

```
O/B    W1    W2    Y1    Y2    G     R     C
 │      │     │     │     │     │     │     │
 │      │     │     │     │     │     │     │
```

Wires from this strip run to the ecobee:

| Air Handler Terminal | ecobee Terminal |
|----------------------|-----------------|
| G                    | G               |
| W1                   | W1              |
| W2                   | W2 (if present) |

The air handler also connects to the heat pump for compressor and reversing
valve signals (Y1, Y2, O/B) — these pass through the air handler board to the
outdoor unit.

---

## R Wire and Power

- The **R wire** from the system goes into the **Rc** terminal on the ecobee.
- **Do not** add a jumper between Rc and Rh — the ecobee does this internally.
- If you have a dual-transformer system (separate transformers for heating and
  cooling), the cooling transformer R goes to Rc and the heating transformer R
  goes to Rh.

---

## O/B Reversing Valve

The O/B wire controls the heat pump's reversing valve, which switches the
refrigerant cycle between heating and cooling modes.

| ecobee Setting | Meaning | Common Brands |
|----------------|---------|---------------|
| **O (Energize on Cool)** | Valve energized during cooling, de-energized during heating | Carrier, Bryant, Lennox, Trane, most brands |
| **B (Energize on Heat)** | Valve energized during heating, de-energized during cooling | Rheem, Ruud |

Configure at: *Settings > Installation Settings > Equipment > Heat Pump > O/B Reversing Valve*

---

## Staging Behavior

| Mode | What Happens |
|------|-------------|
| **Heating — Stage 1** | Heat pump compressor runs (Y1 + O/B) |
| **Heating — Stage 2** | Aux heat strips energize (W1) to supplement heat pump |
| **Heating — Emergency** | Heat pump disabled, aux strips only (W1, optionally W2) |
| **Cooling — Stage 1** | Compressor runs (Y1), reversing valve set to cool |
| **Cooling — Stage 2** | Second-stage compressor (Y2) if equipped |
| **Fan only** | Blower runs (G), no compressor or heat |

---

## Y2 and W2 (Dashed Lines on Diagram)

Y2 (stage 2 compressor) and W2 (stage 2 aux heat) are shown as dashed lines
on the official diagram — they are optional and only apply if:

- The heat pump has a two-stage compressor (Y2)
- The air handler has two banks of heat strips (W2)

If your system is single-stage, leave Y2 and W2 disconnected.

---

## ecobee Configuration Path

After wiring, configure the system type in the ecobee:

1. *Settings > Installation Settings > Equipment*
2. Set system type to **Heat Pump**
3. Configure number of heating stages (compressor)
4. Configure number of cooling stages
5. Configure aux heat stages (W1, optionally W2)
6. Set O/B reversing valve direction (O or B per manufacturer)
7. Set compressor minimum outdoor temperature

---

## Quick Reference

```
ecobee ECB701 Backplate

        LEFT                RIGHT
    ┌──────────┐       ┌──────────┐
    │  Rc ●────│───R───│          │
    │   G ●────│───G───│  Rh ●   │  (internally jumpered to Rc)
    │  Y1 ●────│──Y1───│   C ●───│──C
    │  Y2 ●┄┄┄┄│┄┄Y2┄┄┄│  W1 ●───│──W1
    │ O/B ●────│──O/B───│  W2 ●┄┄┄│┄┄W2
    │          │       │ PEK ●   │
    └──────────┘       └──────────┘

    ── solid  = required connections
    ┄┄ dashed = optional (stage 2, if equipped)
```
