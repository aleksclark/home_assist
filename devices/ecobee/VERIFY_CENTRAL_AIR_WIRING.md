# Verifying Central Air Unit Wiring Configuration

> How to confirm what each thermostat wire actually does at the air handler /
> furnace — independent of wire color or prior labels.

---

## 1. Safety First

| Rule | Detail |
|------|--------|
| **Kill power** | Turn off the furnace / air-handler breaker *and* flip the service disconnect before touching any wires. |
| **Verify dead** | Use a non-contact voltage tester on the high-voltage side before opening panels. |
| **Low-voltage ≠ no-risk** | 24 VAC won't kill you, but a short can blow the 3–5 A fuse on the control board or damage the transformer. |
| **Gas smell → stop** | Leave the area and call the gas company. |

---

## 2. Tools You Need

- **Digital multimeter** (VAC, ohms / continuity modes)
- **Non-contact voltage tester** (for 120/240 V verification)
- **Screwdrivers** (¼″ flat and #2 Phillips are most common)
- **Phone camera** (photograph everything before disconnecting)
- **Masking tape + marker** (for re-labeling wires)
- **Small flashlight / headlamp**

---

## 3. Locate and Document Existing Connections

### 3a. At the Furnace / Air Handler

1. Remove the lower service panel.
2. Find the **control board** — a green circuit board with screw terminals or
   push-in connectors labeled R, C, W, Y, G, O/B, etc.
3. **Photograph the board** with wires still attached — capture every terminal
   label and wire color.
4. Look for a **wiring diagram** — usually printed on the inside of the service
   panel door or on a sticker on the unit itself.

### 3b. At the Thermostat Wall Plate

1. Pull the thermostat off the wall plate.
2. Photograph the wires and their terminal assignments on the plate.
3. Note any wires that are present in the cable bundle but **not connected**
   (tucked into the wall) — one of these may be a usable C wire.

### 3c. Cross-Reference

Compare the two photographs. Each wire color at the thermostat should land on
the **same-lettered** terminal at the furnace. If they don't match, someone
miswired the system.

> **Wire color is convention, not law.** The only thing that matters is which
> terminal each conductor is screwed into at both ends.

---

## 4. Standard Wire Color Conventions (For Reference Only)

| Terminal | Typical Color | Function |
|----------|--------------|----------|
| R        | Red          | 24 V hot from transformer |
| C        | Blue (or Black) | 24 V common / return |
| W        | White        | Heat call (stage 1) |
| W2       | Brown or Light Blue | Heat call (stage 2 / aux) |
| Y        | Yellow       | Cooling / compressor call |
| Y2       | Light Blue or Purple | Cooling stage 2 |
| G        | Green        | Indoor blower / fan |
| O/B      | Orange or Dark Blue | Heat-pump reversing valve |

**Repeat: colors can be wrong. Always verify by terminal, not by color.**

---

## 5. Continuity Testing (Tracing Each Wire)

Use this when you cannot visually confirm which wire at the thermostat
corresponds to which wire at the furnace (e.g., two "white" wires, re-used
colors, or unlabeled splices).

### Procedure

1. **Power off** the furnace breaker.
2. At the **furnace**, disconnect **all** thermostat wires from the control
   board terminals. Spread them apart so no bare ends touch.
3. At the **thermostat** wall plate, disconnect all wires.
4. Pick one wire at the thermostat end. Using a short jumper or alligator clip,
   **short it to one other wire** at the thermostat end (twist bare ends
   together).
5. Go to the furnace end. Set the multimeter to **continuity / ohms**.
6. Touch one probe to the matching-color wire. Touch the other probe to each
   remaining wire in turn.
7. The pair that beeps (or reads < 5 Ω) is the **same pair** you shorted at the
   thermostat.
8. Label both ends with masking tape. Undo the short.
9. Repeat for every conductor in the bundle.

This positively maps every wire regardless of color.

---

## 6. Voltage Testing (Verifying Live Operation)

### 6a. Transformer Output

With the furnace powered on and wires reconnected:

| Probes On      | Expected Reading | Meaning |
|----------------|-----------------|---------|
| R to C         | 24 VAC (±3 V)  | Transformer healthy |
| R to C         | 0 V             | Blown fuse, dead transformer, or open C wire |
| R to C         | < 20 V          | Weak transformer or heavy load / short |

### 6b. Checking a Heat Call

1. Set the thermostat to **Heat** mode, setpoint well above room temperature.
2. At the furnace control board, measure **R to W**.

| Reading | Meaning |
|---------|---------|
| ~24 VAC | Thermostat is actively calling for heat (expected) |
| 0 V     | Thermostat is **not** calling for heat |

3. Now set the thermostat to **Off**.
4. Measure R to W again.

| Reading | Meaning |
|---------|---------|
| 0 V     | Normal — no call |
| ~24 VAC | **Problem** — voltage present when it shouldn't be (see wiring issues doc) |

### 6c. Checking a Cooling Call

Same process: set thermostat to Cool, setpoint well below room temp, measure
**R to Y**. Should see ~24 VAC only during an active cooling call.

### 6d. Fan Call

Set thermostat fan to **On** (not Auto). Measure **R to G** — should see
~24 VAC.

---

## 7. Testing for Wire-to-Wire Shorts

A short between wires (especially R to W) is a top cause of heat running
when not called for.

### Procedure

1. **Power off** furnace.
2. **Disconnect all wires** at both the thermostat and furnace ends.
3. Set multimeter to **ohms (Ω)**.
4. At one end of the cable, measure resistance between **every pair** of wires:

| Pair     | Expected   | Problem If |
|----------|-----------|------------|
| R ↔ W    | Open (∞)  | < 5 Ω → short causing constant heat |
| R ↔ Y    | Open (∞)  | < 5 Ω → short causing constant cool |
| R ↔ G    | Open (∞)  | < 5 Ω → short causing fan always on |
| W ↔ C    | Open (∞)  | < 5 Ω → short, potential control board damage |
| Any pair  | Open (∞)  | Low resistance = insulation damage somewhere in the run |

If you find a short, the cable needs to be repaired or replaced.

---

## 8. Reading the Furnace Control Board

### LED Diagnostic Codes

Most modern furnace boards have a **status LED** that blinks a pattern. The
blink code chart is printed on the inside of the service panel door.

| Common Patterns | Typical Meaning |
|----------------|-----------------|
| Steady ON      | Normal — power present, no call |
| Slow blink     | Normal — standby |
| Rapid blink    | Call for heat active |
| 2 blinks, pause | Pressure switch error |
| 3 blinks, pause | Ignition failure |
| 4 blinks, pause | Open high-limit switch |
| Continuous ON, no blink | Board may be locked out — power-cycle to reset |

Consult your specific furnace manual — codes vary by manufacturer.

### Fuses

- Look for a **3 A or 5 A glass fuse** on the control board.
- A blown fuse (blackened, broken filament) means a short circuit occurred.
- Replace with the **exact same amperage** — never upsize.
- If the new fuse blows immediately, a short still exists in the wiring or a
  component has failed.

---

## 9. Common Wiring Configurations

### Heat-Only (2-Wire)

```
Thermostat        Furnace
   R  ──────────── R
   W  ──────────── W
```

### Standard Heat + Cool (4-Wire, No C)

```
Thermostat        Furnace
   R  ──────────── R
   W  ──────────── W
   Y  ──────────── Y
   G  ──────────── G
```

### Standard Heat + Cool + C (5-Wire)

```
Thermostat        Furnace
   R  ──────────── R
   C  ──────────── C
   W  ──────────── W
   Y  ──────────── Y
   G  ──────────── G
```

### Heat Pump (6–8 Wire)

```
Thermostat        Furnace / Air Handler
   R  ──────────── R
   C  ──────────── C
   Y  ──────────── Y   (compressor)
   G  ──────────── G   (fan)
   O/B ─────────── O/B (reversing valve)
   W2 ──────────── W2  (aux / emergency heat)
```

---

## 10. When to Call a Professional

- Transformer reads 0 V and breaker is on.
- Control board shows burn marks or corroded traces.
- You find a short but cannot access the wire run to repair it.
- Gas valve or ignition components need testing.
- You are unsure about any measurement or uncomfortable working near gas lines.
- CO detector alarms during testing.

---

## References

- [PickHVAC — Thermostat Wiring Guide](https://www.pickhvac.com/thermostat/wiring/)
- [PickHVAC — Furnace Control Board Troubleshooting](https://www.pickhvac.com/furnace/troubleshoot/furnace-control-board-ultimate-troubleshooting-guide/)
- [Wiring URU — Thermostat Wiring Diagrams](https://wiringuru.com/thermostat-wiring-diagram/)
- [About Darwin — Thermostat Wiring](https://aboutdarwin.com/thermostat-wiring/)
