import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.components import climate, uart, select, switch
from esphome.const import CONF_ID

CODEOWNERS = ["@aleksclark"]
DEPENDENCIES = ["uart"]
AUTO_LOAD = ["select", "switch"]

tcl_ns = cg.esphome_ns.namespace("tcl_climate")
TCLClimate = tcl_ns.class_(
    "TCLClimate", climate.Climate, uart.UARTDevice, cg.PollingComponent
)
TCLSwingSelect = tcl_ns.class_("TCLSwingSelect", select.Select, cg.Component)
TCLSwitch = tcl_ns.class_("TCLSwitch", switch.Switch, cg.Component)

CONF_VERTICAL_SWING = "vertical_swing"
CONF_HORIZONTAL_SWING = "horizontal_swing"
CONF_BUZZER_SWITCH = "buzzer"
CONF_DISPLAY_SWITCH = "display"

CONFIG_SCHEMA = (
    climate.climate_schema(TCLClimate)
    .extend(
        {
            cv.Optional(CONF_VERTICAL_SWING): select.select_schema(TCLSwingSelect),
            cv.Optional(CONF_HORIZONTAL_SWING): select.select_schema(TCLSwingSelect),
            cv.Optional(CONF_BUZZER_SWITCH): switch.switch_schema(TCLSwitch),
            cv.Optional(CONF_DISPLAY_SWITCH): switch.switch_schema(TCLSwitch),
        }
    )
    .extend(uart.UART_DEVICE_SCHEMA)
    .extend(cv.polling_component_schema("500ms"))
)


async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
    await climate.register_climate(var, config)
    await uart.register_uart_device(var, config)

    if CONF_VERTICAL_SWING in config:
        swing_v = await select.new_select(
            config[CONF_VERTICAL_SWING],
            options=[
                "none",
                "move_full",
                "move_upper",
                "move_lower",
                "fix_top",
                "fix_upper",
                "fix_mid",
                "fix_lower",
                "fix_bottom",
            ],
        )
        cg.add(var.set_vertical_swing_select(swing_v))

    if CONF_HORIZONTAL_SWING in config:
        swing_h = await select.new_select(
            config[CONF_HORIZONTAL_SWING],
            options=[
                "none",
                "move_full",
                "move_left",
                "move_mid",
                "move_right",
                "fix_left",
                "fix_mid_left",
                "fix_mid",
                "fix_mid_right",
                "fix_right",
            ],
        )
        cg.add(var.set_horizontal_swing_select(swing_h))

    if CONF_BUZZER_SWITCH in config:
        buzzer = await switch.new_switch(config[CONF_BUZZER_SWITCH])
        cg.add(var.set_buzzer_switch(buzzer))

    if CONF_DISPLAY_SWITCH in config:
        display = await switch.new_switch(config[CONF_DISPLAY_SWITCH])
        cg.add(var.set_display_switch(display))
