# Ecobee ECB701 (Smart Thermostat Essential) — Known Wiring Issues

> **Model:** ECB701 / EB-STATE7-01 — ecobee Smart Thermostat Essential  
> **Problem:** Heat runs constantly even when the thermostat is not calling for it  
> **PEK Status:** Not using Power Extender Kit

---

## 1. Model-Specific Details

The ECB701 is ecobee's entry-level smart thermostat. Its backplate exposes the
following terminals:

| Terminal | Function                  |
|----------|---------------------------|
| **R**    | 24 V power (from transformer) |
| **C**    | Common (return to transformer) |
| **W**    | Stage 1 heat              |
| **W2**   | Stage 2 / auxiliary heat   |
| **Y**    | Stage 1 cooling (compressor) |
| **Y2**   | Stage 2 cooling            |
| **G**    | Indoor fan / blower        |
| **O/B**  | Heat-pump reversing valve  |
| **PEK**  | Power Extender Kit connector |

The Essential model does **not** have a dedicated Rh/Rc split — there is a
single **R** terminal. If the original system had separate Rh and Rc wires, both
should land on R (the ecobee internally jumpers heating and cooling power rails).

---

## 2. Root Causes — Heat Running When Not Called For

### 2a. Stuck Internal Relay (~40 % of hardware failures)

The W-terminal relay inside the ecobee can fail in the **closed** position,
continuously sending 24 VAC to the furnace regardless of what the screen shows.

**Signature:**  
- ecobee display says "Off" or "Cooling," but furnace fires.  
- Removing the ecobee from the wall plate **immediately stops** the heat.

**Fix:** Warranty replacement through ecobee.

---

### 2b. Shorted W Wire to R Wire (~40 % of hardware failures)

If the W (white) conductor is shorted to the R (red) conductor anywhere in the
wire run — from staples, pinched insulation, rodent damage, or a bad splice —
the furnace receives a permanent heat-call that **bypasses the thermostat
entirely**.

**Signature:**  
- Heat runs even with the ecobee completely removed from the wall plate.  
- Heat runs even if the W wire is disconnected at the thermostat end.  
- Heat **stops** only when the W wire is disconnected at the furnace control
  board.

**Fix:** Locate and repair the short, or pull a new thermostat cable.

---

### 2c. Stuck Furnace Control Board Relay (~15 % of hardware failures)

The relay on the furnace's own control board can weld shut. The board then keeps
the gas valve / heat strips energized no matter what signal arrives on W.

**Signature:**  
- Disconnecting W at the **furnace** control board does **not** stop the heat.  
- Ecobee shows no heat call.

**Fix:** Replace the furnace control board ($250–600 parts + labor).

---

### 2d. Incorrect Wiring at Installation

| Mistake | Effect |
|---------|--------|
| W and Y wires swapped | Heat runs when cooling is called and vice-versa |
| W wire on **W2** instead of **W** | Stage 2 / aux heat fires unexpectedly or continuously |
| Loose W wire in terminal | Intermittent relay chatter; furnace may latch on |
| R wire on wrong terminal at furnace | Continuous 24 V on W circuit |
| Extra bare copper touching adjacent terminal | Creates accidental cross-connection |

---

### 2e. Missing C Wire (Indirect Cause)

Without a C wire and without the PEK, the ecobee must "power-steal" from the
call circuits. This does **not** directly cause constant heating, but it can:

- Cause intermittent thermostat reboots mid-cycle.
- Produce erratic relay behavior that the furnace interprets as a sustained call.
- Drop voltage enough that the furnace control board misreads the W signal.

**Recommendation:** If no C wire is available, install the PEK or run a new
cable. The ECB701 ships without a PEK — it must be purchased separately
(ecobee part PEK-01).

---

### 2f. Software / Firmware Glitches (~30 % of all "won't turn off" reports)

- ecobee freezes with "Heating" displayed.
- A firmware update changes relay state mid-cycle.
- Smart Recovery pre-heats 30–60 min before a schedule transition, which can
  look like constant heating.

**Fix:**  
1. Force reboot: pull thermostat off wall plate for 30 seconds, re-seat.  
2. Disable Smart Recovery: *Settings → Preferences → Smart Recovery → Off*.  
3. Update firmware: *Settings → About* — install any pending update.  
4. Factory reset (last resort): *Settings → Reset → Reset All Settings*.

---

### 2g. Aggressive Threshold / Staging Settings

| Setting | Default | Symptom | Recommended |
|---------|---------|---------|-------------|
| Heat differential | 0.5 °F | Constant short-cycling that appears as non-stop heat | 1.0–1.5 °F |
| Stage 2 heat differential | 1.0 °F | Aux heat fires too easily | 2.5–3.0 °F |
| Compressor min cycle time | 5 min | — | Leave at default unless advised by HVAC tech |

Path: *Settings → Installation Settings → Thresholds*

---

### 2h. Temperature Sensor Reads Low

If cold air leaks behind the wall plate and reaches the internal sensor, the
ecobee perpetually thinks the room is colder than it is and keeps calling for
heat.

**Fix:**  
- Seal the wall opening behind the thermostat with foam or putty.  
- Apply a temperature correction offset:  
  *Settings → Installation Settings → Thresholds → Temperature Correction*  
  (adjust in 0.5 °F increments, compare with a known-accurate thermometer).

---

## 3. Quick Isolation Test (The "Pull It Off the Wall" Test)

```text
1.  Set ecobee to OFF.
    └─ Heat stops within 1–2 min?
         YES → software / config issue (reboot, check settings)
         NO  → continue

2.  Remove ecobee from wall plate.
    └─ Heat stops immediately?
         YES → stuck ecobee relay (warranty replacement)
         NO  → continue

3.  At the thermostat wall plate, disconnect the W wire and cap it.
    └─ Heat stops?
         YES → ecobee was sending signal; likely config or relay issue
         NO  → continue

4.  At the furnace control board, disconnect the W wire.
    └─ Heat stops?
         YES → short in the W-wire run (repair / re-pull cable)
         NO  → stuck furnace relay (replace control board)
```

---

## 4. References

- [ecobee Troubleshooting — Keeps Calling for Heat](https://www.s3semi.com/ecobee-keeps-calling-for-heat-complete-troubleshooting-guide/)
- [ecobee Not Turning Off Heat](https://www.s3semi.com/ecobee-not-turning-off-heat-complete-troubleshooting-guide/)
- [PickComfort — Ecobee Furnace Troubleshooting](https://www.pickcomfort.com/ecobee-troubleshooting-furnace-boiler-heating-systems/)
- [B&H — ECB701 Specifications](https://www.bhphotovideo.com/c/product/1885643-REG/ecobee_eb_state7_01_smart_thermostat_essential.html)
