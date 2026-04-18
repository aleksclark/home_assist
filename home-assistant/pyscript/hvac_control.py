"""
HVAC zone control — pyscript.

Zones: kitchen, amos (mini-splits), hallway (Matter thermostat), livingroom (mini-split).
Schedule: early_morning 05:30-06:30, morning 06:30-08:30, daytime 08:30-21:00, overnight 21:00-05:30.
Setpoints from input_number.hvac_{period}_{heat,cool}_f.

Rules:
  - heat_f is the low boundary: if temp drops below it, heat to heat_f.
  - cool_f is the high boundary: if temp rises above it, cool to cool_f.
  - Between heat_f and cool_f: turn off (comfort zone).
  - Hallway thermostat always heat_cool with period setpoints.
  - Livingroom: same but offset (heat to heat_f-5, cool to cool_f+5).
  - 30s debounce. Unavailable skipped. No-op if state already matches.
  - Displays off 8pm-8am.
"""

import time as time_mod
from datetime import datetime as dt_now_cls

_last_cmd = {}
_last_display = {}
DEBOUNCE_S = 30
LR_OFFSET = 5
DISPLAY_OFF_START = 20
DISPLAY_OFF_END = 8

MINISPLIT_UNITS = ["minisplit_kitchen", "minisplit_livingroom", "minisplit_amos"]


def _now_period():
    mins = dt_now_cls.now().hour * 60 + dt_now_cls.now().minute
    if 330 <= mins < 390:
        return "early_morning"
    elif 390 <= mins < 510:
        return "morning"
    elif 510 <= mins < 1260:
        return "daytime"
    else:
        return "overnight"


def _sync_period():
    period = _now_period()
    if state.get("input_select.hvac_schedule_period") != period:
        input_select.select_option(entity_id="input_select.hvac_schedule_period", option=period)
    return period


def _debounced(eid):
    return (time_mod.monotonic() - _last_cmd.get(eid, 0)) < DEBOUNCE_S


def _mark(eid):
    _last_cmd[eid] = time_mod.monotonic()


def _ok(eid):
    s = state.get(eid)
    return s is not None and s not in ("unavailable", "unknown")


def _setpoints(period):
    h = float(state.get(f"input_number.hvac_{period}_heat_f") or 65)
    c = float(state.get(f"input_number.hvac_{period}_cool_f") or 75)
    return h, c


def _control_minisplit(eid, zone, mqtt_unit, heat_sp, cool_sp):
    """
    Below heat_sp: heat to heat_sp.
    Above cool_sp: cool to cool_sp.
    Between: off.
    """
    if not _ok(eid) or _debounced(eid):
        return
    if state.get("input_boolean.hvac_master_enable") != "on":
        return

    attrs = state.getattr(eid)
    temp = attrs.get("current_temperature")
    if temp is None:
        return
    temp = float(temp)
    cur_mode = state.get(eid)
    cur_target = attrs.get("temperature")

    if temp < heat_sp:
        want_mode = "heat"
        want_temp = heat_sp
    elif temp > cool_sp:
        want_mode = "cool"
        want_temp = cool_sp
    else:
        want_mode = "off"
        want_temp = None

    # Check if already correct
    if want_mode == "off":
        if cur_mode == "off":
            return
    else:
        if cur_mode == want_mode and cur_target is not None and float(cur_target) == want_temp:
            return

    if want_mode == "off":
        log.info(f"hvac {zone}: {temp}F in [{heat_sp},{cool_sp}] -> off")
        climate.set_hvac_mode(entity_id=eid, hvac_mode="off")
    else:
        log.info(f"hvac {zone}: {temp}F -> {want_mode} @ {want_temp}F")
        climate.set_temperature(entity_id=eid, temperature=want_temp, hvac_mode=want_mode)
        mqtt.publish(topic=f"cmnd/{mqtt_unit}/Buzzer", payload="0")
    _mark(eid)


def _control_hallway():
    eid = "climate.smart_thermostat"
    if not _ok(eid) or _debounced(eid):
        return
    if state.get("input_boolean.hvac_master_enable") != "on":
        return

    period = _sync_period()
    heat_sp, cool_sp = _setpoints(period)
    attrs = state.getattr(eid)

    if (state.get(eid) == "heat_cool"
            and attrs.get("target_temp_low") is not None
            and float(attrs.get("target_temp_low")) == heat_sp
            and attrs.get("target_temp_high") is not None
            and float(attrs.get("target_temp_high")) == cool_sp):
        return

    log.info(f"hvac hallway: -> heat_cool {heat_sp}/{cool_sp}")
    climate.set_temperature(entity_id=eid, hvac_mode="heat_cool", target_temp_low=heat_sp, target_temp_high=cool_sp)
    _mark(eid)


def _master_off():
    for eid in ["climate.kitchen_ac", "climate.livingroom_ac", "climate.amos_bedroom_ac", "climate.smart_thermostat"]:
        if _ok(eid) and state.get(eid) != "off":
            climate.set_hvac_mode(entity_id=eid, hvac_mode="off")
            _mark(eid)
    log.info("hvac: master disable -> all off")


def _sync_displays():
    hour = dt_now_cls.now().hour
    want = "0" if (hour >= DISPLAY_OFF_START or hour < DISPLAY_OFF_END) else "1"
    for unit in MINISPLIT_UNITS:
        if _last_display.get(unit) != want:
            mqtt.publish(topic=f"cmnd/{unit}/Display", payload=want)
            _last_display[unit] = want
            log.info(f"hvac display {unit}: -> {'off' if want == '0' else 'on'}")


def _all_zones():
    period = _sync_period()
    heat_sp, cool_sp = _setpoints(period)

    _control_minisplit("climate.kitchen_ac", "kitchen", "minisplit_kitchen", heat_sp, cool_sp)
    _control_minisplit("climate.amos_bedroom_ac", "amos", "minisplit_amos", heat_sp, cool_sp)
    _control_minisplit("climate.livingroom_ac", "livingroom", "minisplit_livingroom",
                       heat_sp - LR_OFFSET, cool_sp + LR_OFFSET)
    _control_hallway()
    _sync_displays()


# ── Triggers ─────────────────────────────────────────────────────────

log.warning("hvac_control.py loaded")


@time_trigger("cron(30 5 * * *)", "cron(30 6 * * *)", "cron(0 8 * * *)", "cron(30 8 * * *)", "cron(0 20 * * *)", "cron(0 21 * * *)")
def hvac_schedule_tick():
    _all_zones()


@time_trigger("cron(*/5 * * * *)")
def hvac_poll():
    _all_zones()


@state_trigger(
    "input_number.hvac_early_morning_heat_f",
    "input_number.hvac_early_morning_cool_f",
    "input_number.hvac_morning_heat_f",
    "input_number.hvac_morning_cool_f",
    "input_number.hvac_daytime_heat_f",
    "input_number.hvac_daytime_cool_f",
    "input_number.hvac_overnight_heat_f",
    "input_number.hvac_overnight_cool_f",
)
def hvac_setpoint_changed(**kwargs):
    _all_zones()


@state_trigger("input_boolean.hvac_master_enable == 'off'")
def hvac_master_disabled(**kwargs):
    _master_off()


@event_trigger("homeassistant_started")
def hvac_ha_started(**kwargs):
    log.info("hvac: HA started")
    _sync_period()
    task.sleep(10)
    _all_zones()
