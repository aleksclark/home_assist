#include "tcl_climate.h"
#include "esphome/core/log.h"

namespace esphome {
namespace tcl_climate {

// --- TCLSwingSelect ---

void TCLSwingSelect::control(const std::string &value) {
  if (this->parent_ == nullptr)
    return;
  this->publish_state(value);
  if (this->is_vertical_) {
    for (uint8_t i = 0; i < sizeof(VSWING_NAMES) / sizeof(VSWING_NAMES[0]); i++) {
      if (value == VSWING_NAMES[i]) {
        this->parent_->set_vertical_swing(static_cast<VerticalSwing>(i));
        return;
      }
    }
  } else {
    for (uint8_t i = 0; i < sizeof(HSWING_NAMES) / sizeof(HSWING_NAMES[0]); i++) {
      if (value == HSWING_NAMES[i]) {
        this->parent_->set_horizontal_swing(static_cast<HorizontalSwing>(i));
        return;
      }
    }
  }
}

// --- TCLSwitch ---

void TCLSwitch::write_state(bool state) {
  if (this->parent_ == nullptr)
    return;
  this->publish_state(state);
  if (this->is_buzzer_)
    this->parent_->set_buzzer(state);
  else
    this->parent_->set_display(state);
}

// --- Checksum helpers ---

uint8_t TCLClimate::xor_checksum_(const uint8_t *buf, int len) {
  uint8_t x = 0;
  for (int i = 0; i < len; i++)
    x ^= buf[i];
  return x;
}

bool TCLClimate::validate_xor_(const uint8_t *buf, int len) {
  if (len < 2)
    return false;
  return xor_checksum_(buf, len - 1) == buf[len - 1];
}

// --- Packet framing (state machine, same logic as OpenBeken) ---

int TCLClimate::frame_byte_(int ch, uint8_t *buf, int buf_len) {
  if (ch < 0)
    return -1;

  if (ch == TCL_HEADER && this->rx_skip_ == 0 && !this->rx_wait_len_) {
    this->rx_pos_ = 0;
    this->rx_skip_ = 3;
    this->rx_wait_len_ = true;
    if (this->rx_pos_ < buf_len)
      buf[this->rx_pos_++] = (uint8_t) ch;
  } else if (this->rx_skip_ == 0 && this->rx_wait_len_) {
    if (this->rx_pos_ < buf_len)
      buf[this->rx_pos_++] = (uint8_t) ch;
    this->rx_skip_ = ch + 1;
    this->rx_wait_len_ = false;
  } else if (this->rx_skip_ > 0) {
    if (this->rx_pos_ < buf_len)
      buf[this->rx_pos_++] = (uint8_t) ch;
    if (--this->rx_skip_ == 0 && !this->rx_wait_len_)
      return this->rx_pos_;
  }

  return -1;
}

// --- Build set command from current + desired state ---

void TCLClimate::build_set_cmd_(GetResponse *resp) {
  memcpy(this->set_cmd_.raw, SET_CMD_BASE, SET_CMD_LEN);

  auto &dst = this->set_cmd_.data;
  auto &src = resp->data;

  dst.power = src.power;
  dst.off_timer_en = 0;
  dst.on_timer_en = 0;
  dst.beep = this->buzzer_on_ ? 1 : 0;
  dst.disp = this->display_on_ ? 1 : 0;
  dst.eco = 0;
  dst.turbo = src.turbo;
  dst.mute = src.mute;

  // Mode mapping: get response mode -> set command mode
  switch (src.mode) {
    case 0x01: dst.mode = 0x03; break;  // cool
    case 0x02: dst.mode = 0x07; break;  // fan_only
    case 0x03: dst.mode = 0x02; break;  // dry
    case 0x04: dst.mode = 0x01; break;  // heat
    case 0x05: dst.mode = 0x08; break;  // auto
    default: dst.mode = 0x03; break;
  }

  // Temperature: get response stores (temp - 16), set uses (15 - get_temp)
  dst.temp = 15 - src.temp;

  // Fan speed mapping: get -> set
  switch (src.fan) {
    case 0x00: dst.fan = 0x00; break;  // auto
    case 0x01: dst.fan = 0x02; break;  // speed 1
    case 0x02: dst.fan = 0x03; break;  // speed 3
    case 0x03: dst.fan = 0x05; break;  // speed 5
    case 0x04: dst.fan = 0x06; break;  // speed 2
    case 0x05: dst.fan = 0x07; break;  // speed 4
    default: dst.fan = 0x00; break;
  }

  // Vertical swing
  if (src.vswing_mv) {
    dst.vswing = 0x07;
    dst.vswing_fix = 0;
    dst.vswing_mv = src.vswing_mv;
  } else if (src.vswing_fix) {
    dst.vswing = 0;
    dst.vswing_fix = src.vswing_fix;
    dst.vswing_mv = 0;
  }

  // Horizontal swing
  if (src.hswing_mv) {
    dst.hswing = 0x01;
    dst.hswing_fix = 0;
    dst.hswing_mv = src.hswing_mv;
  } else if (src.hswing_fix) {
    dst.hswing = 0;
    dst.hswing_fix = src.hswing_fix;
    dst.hswing_mv = 0;
  }

  dst.half_degree = 0;
  dst.byte_7_bit_0_1 = 0;

  // XOR checksum
  this->set_cmd_.raw[SET_CMD_LEN - 1] = xor_checksum_(this->set_cmd_.raw, SET_CMD_LEN - 1);
}

// --- Send helpers ---

void TCLClimate::send_set_cmd_() {
  this->write_array(this->set_cmd_.raw, SET_CMD_LEN);
  ESP_LOGD(TAG, "Sent SET command (%d bytes)", SET_CMD_LEN);
}

void TCLClimate::send_poll_() {
  this->write_array(REQ_CMD, sizeof(REQ_CMD));
}

// --- Parse a complete 61-byte response ---

void TCLClimate::parse_response_(uint8_t *buf, int len) {
  if (len != GET_RESP_LEN || buf[3] != 0x04)
    return;

  if (!validate_xor_(buf, len)) {
    ESP_LOGW(TAG, "Bad checksum, ignoring packet");
    return;
  }

  memcpy(this->last_resp_.raw, buf, len);
  this->got_first_response_ = true;
  auto &d = this->last_resp_.data;

  bool changed = false;

  // --- Climate mode ---
  climate::ClimateMode new_mode;
  if (this->heat_cool_mode_) {
    // In heat_cool mode, always report HEAT_COOL to HA regardless of
    // what the hardware is actually doing (heat or cool).
    new_mode = climate::CLIMATE_MODE_HEAT_COOL;
  } else if (d.power == 0) {
    new_mode = climate::CLIMATE_MODE_OFF;
  } else {
    switch (d.mode) {
      case 0x01: new_mode = climate::CLIMATE_MODE_COOL; break;
      case 0x02: new_mode = climate::CLIMATE_MODE_FAN_ONLY; break;
      case 0x03: new_mode = climate::CLIMATE_MODE_DRY; break;
      case 0x04: new_mode = climate::CLIMATE_MODE_HEAT; break;
      case 0x05: new_mode = climate::CLIMATE_MODE_AUTO; break;
      default: new_mode = climate::CLIMATE_MODE_OFF; break;
    }
  }
  if (this->mode != new_mode) {
    this->mode = new_mode;
    changed = true;
  }

  // --- Fan mode (custom strings to match OpenBeken labels) ---
  std::string new_fan;
  if (d.turbo) {
    new_fan = "turbo";
  } else if (d.mute) {
    new_fan = "mute";
  } else {
    switch (d.fan) {
      case 0x00: new_fan = "auto"; break;
      case 0x01: new_fan = "1"; break;
      case 0x04: new_fan = "2"; break;
      case 0x02: new_fan = "3"; break;
      case 0x05: new_fan = "4"; break;
      case 0x03: new_fan = "5"; break;
      default: new_fan = "auto"; break;
    }
  }
  if (!this->has_custom_fan_mode() || this->get_custom_fan_mode() != new_fan) {
    this->set_custom_fan_mode_(new_fan.c_str());
    changed = true;
  }

  // --- Target temperature ---
  float new_target = (float) (d.temp + 16);
  if (this->target_temperature != new_target) {
    this->target_temperature = new_target;
    changed = true;
  }

  // --- Current temperature (from bytes 17-18) ---
  float new_current = (((buf[17] << 8) | buf[18]) / 374.0f - 32.0f) / 1.8f;
  if (std::abs(this->current_temperature - new_current) > 0.1f) {
    this->current_temperature = new_current;
    changed = true;
  }

  // In heat_cool mode, re-evaluate on every temp update
  if (this->heat_cool_mode_) {
    this->target_temperature_low = this->heat_cool_low_;
    this->target_temperature_high = this->heat_cool_high_;
    this->apply_heat_cool_logic_();
  }

  // --- Vertical swing position ---
  VerticalSwing new_vs = VS_NONE;
  if (d.vswing_mv == 0x01) new_vs = VS_MOVE_FULL;
  else if (d.vswing_mv == 0x02) new_vs = VS_MOVE_UPPER;
  else if (d.vswing_mv == 0x03) new_vs = VS_MOVE_LOWER;
  else if (d.vswing_fix == 0x01) new_vs = VS_FIX_TOP;
  else if (d.vswing_fix == 0x02) new_vs = VS_FIX_UPPER;
  else if (d.vswing_fix == 0x03) new_vs = VS_FIX_MID;
  else if (d.vswing_fix == 0x04) new_vs = VS_FIX_LOWER;
  else if (d.vswing_fix == 0x05) new_vs = VS_FIX_BOTTOM;

  if (new_vs != this->vswing_) {
    this->vswing_ = new_vs;
    if (this->vswing_select_ != nullptr)
      this->vswing_select_->publish_state(VSWING_NAMES[new_vs]);
  }

  // --- Horizontal swing position ---
  HorizontalSwing new_hs = HS_NONE;
  if (d.hswing_mv == 0x01) new_hs = HS_MOVE_FULL;
  else if (d.hswing_mv == 0x02) new_hs = HS_MOVE_LEFT;
  else if (d.hswing_mv == 0x03) new_hs = HS_MOVE_MID;
  else if (d.hswing_mv == 0x04) new_hs = HS_MOVE_RIGHT;
  else if (d.hswing_fix == 0x01) new_hs = HS_FIX_LEFT;
  else if (d.hswing_fix == 0x02) new_hs = HS_FIX_MID_LEFT;
  else if (d.hswing_fix == 0x03) new_hs = HS_FIX_MID;
  else if (d.hswing_fix == 0x04) new_hs = HS_FIX_MID_RIGHT;
  else if (d.hswing_fix == 0x05) new_hs = HS_FIX_RIGHT;

  if (new_hs != this->hswing_) {
    this->hswing_ = new_hs;
    if (this->hswing_select_ != nullptr)
      this->hswing_select_->publish_state(HSWING_NAMES[new_hs]);
  }

  // --- Buzzer / Display state feedback ---
  bool buzzer_state = d.disp;  // note: in GET response there's no beep field, only disp
  bool display_state = (d.disp != 0);
  if (this->display_switch_ != nullptr) {
    if (this->display_switch_->state != display_state)
      this->display_switch_->publish_state(display_state);
  }

  if (changed) {
    ESP_LOGD(TAG, "State: mode=%d target=%.0f current=%.1f fan=%s",
             (int) this->mode, this->target_temperature, this->current_temperature,
             new_fan.c_str());
    this->publish_state();
  }
}

// --- Component lifecycle ---

void TCLClimate::setup() {
  ESP_LOGCONFIG(TAG, "Setting up TCL Climate...");
  this->rx_pos_ = 0;
  this->rx_wait_len_ = false;
  this->rx_skip_ = 0;
}

void TCLClimate::dump_config() {
  ESP_LOGCONFIG(TAG, "TCL Climate:");
  ESP_LOGCONFIG(TAG, "  Poll interval: %u ms", this->get_update_interval());
}

void TCLClimate::loop() {
  this->process_rx_();
}

void TCLClimate::process_rx_() {
  uint8_t rx_buf[MAX_RX_LEN];
  int avail = this->available();
  if (avail > 0 && !this->rx_logged_) {
    ESP_LOGD(TAG, "RX: %d bytes available", avail);
    this->rx_logged_ = true;
  }

  while (this->available()) {
    uint8_t byte;
    this->read_byte(&byte);
    int len = this->frame_byte_((int) byte, rx_buf, MAX_RX_LEN);
    if (len > 0) {
      ESP_LOGD(TAG, "RX frame complete: %d bytes, hdr=0x%02X type=0x%02X", len, rx_buf[0], len > 3 ? rx_buf[3] : 0);
      this->parse_response_(rx_buf, len);
    }
  }
}

void TCLClimate::update() {
  if (this->pending_send_) {
    this->pending_send_ = false;
    this->send_set_cmd_();
  } else {
    this->send_poll_();
  }
}

// --- Climate traits ---

climate::ClimateTraits TCLClimate::traits() {
  auto traits = climate::ClimateTraits();
  traits.add_feature_flags(climate::CLIMATE_SUPPORTS_CURRENT_TEMPERATURE);
  traits.add_feature_flags(climate::CLIMATE_SUPPORTS_TWO_POINT_TARGET_TEMPERATURE);
  traits.set_supported_modes({
      climate::CLIMATE_MODE_OFF,
      climate::CLIMATE_MODE_COOL,
      climate::CLIMATE_MODE_HEAT,
      climate::CLIMATE_MODE_HEAT_COOL,
      climate::CLIMATE_MODE_FAN_ONLY,
      climate::CLIMATE_MODE_DRY,
      climate::CLIMATE_MODE_AUTO,
  });
  traits.set_supported_custom_fan_modes({
      "auto", "1", "2", "3", "4", "5", "mute", "turbo",
  });
  traits.set_supported_swing_modes({
      climate::CLIMATE_SWING_OFF,
      climate::CLIMATE_SWING_BOTH,
      climate::CLIMATE_SWING_VERTICAL,
      climate::CLIMATE_SWING_HORIZONTAL,
  });
  traits.set_visual_min_temperature(16.0f);
  traits.set_visual_max_temperature(31.0f);
  traits.set_visual_target_temperature_step(1.0f);
  return traits;
}

// --- Control (HA -> device) ---

void TCLClimate::control(const climate::ClimateCall &call) {
  if (!this->got_first_response_) {
    ESP_LOGW(TAG, "No response from AC yet, ignoring command");
    return;
  }

  if (call.get_mode().has_value()) {
    auto m = *call.get_mode();
    if (m == climate::CLIMATE_MODE_HEAT_COOL) {
      // Enter dual-setpoint mode — don't send to hardware yet,
      // apply_heat_cool_logic_() will pick heat or cool on next poll.
      this->heat_cool_mode_ = true;
      this->mode = climate::CLIMATE_MODE_HEAT_COOL;
      // Accept any setpoints provided in the same call
      if (call.get_target_temperature_low().has_value())
        this->heat_cool_low_ = *call.get_target_temperature_low();
      if (call.get_target_temperature_high().has_value())
        this->heat_cool_high_ = *call.get_target_temperature_high();
      this->target_temperature_low = this->heat_cool_low_;
      this->target_temperature_high = this->heat_cool_high_;
      this->publish_state();
      // Immediately evaluate which sub-mode to use
      this->apply_heat_cool_logic_();
      return;
    } else {
      // Explicit mode selection exits heat_cool
      this->heat_cool_mode_ = false;
    }
  }

  // Update dual setpoints even if mode wasn't changed (slider adjustment)
  if (this->heat_cool_mode_) {
    bool changed = false;
    if (call.get_target_temperature_low().has_value()) {
      this->heat_cool_low_ = *call.get_target_temperature_low();
      changed = true;
    }
    if (call.get_target_temperature_high().has_value()) {
      this->heat_cool_high_ = *call.get_target_temperature_high();
      changed = true;
    }
    if (changed) {
      this->target_temperature_low = this->heat_cool_low_;
      this->target_temperature_high = this->heat_cool_high_;
      this->publish_state();
      this->apply_heat_cool_logic_();
      return;
    }
  }

  GetResponse working{};
  memcpy(working.raw, this->last_resp_.raw, GET_RESP_LEN);

  if (call.get_mode().has_value()) {
    auto m = *call.get_mode();
    if (m == climate::CLIMATE_MODE_OFF) {
      working.data.power = 0;
    } else {
      working.data.power = 1;
      switch (m) {
        case climate::CLIMATE_MODE_COOL: working.data.mode = 0x01; break;
        case climate::CLIMATE_MODE_DRY: working.data.mode = 0x03; break;
        case climate::CLIMATE_MODE_FAN_ONLY: working.data.mode = 0x02; break;
        case climate::CLIMATE_MODE_HEAT: working.data.mode = 0x04; break;
        case climate::CLIMATE_MODE_AUTO: working.data.mode = 0x05; break;
        default: break;
      }
    }
  }

  if (call.get_target_temperature().has_value()) {
    float t = *call.get_target_temperature();
    working.data.temp = (uint8_t) t - 16;
  }

  if (call.has_custom_fan_mode()) {
    auto fan = std::string(call.get_custom_fan_mode().c_str(), call.get_custom_fan_mode().size());
    working.data.turbo = 0;
    working.data.mute = 0;
    if (fan == "turbo") {
      working.data.fan = 0x03;
      working.data.turbo = 1;
    } else if (fan == "mute") {
      working.data.fan = 0x01;
      working.data.mute = 1;
    } else if (fan == "auto") {
      working.data.fan = 0x00;
    } else if (fan == "1") {
      working.data.fan = 0x01;
    } else if (fan == "2") {
      working.data.fan = 0x04;
    } else if (fan == "3") {
      working.data.fan = 0x02;
    } else if (fan == "4") {
      working.data.fan = 0x05;
    } else if (fan == "5") {
      working.data.fan = 0x03;
    }
  }

  if (call.get_swing_mode().has_value()) {
    auto sw = *call.get_swing_mode();
    working.data.vswing_mv = 0;
    working.data.vswing_fix = 0;
    working.data.hswing_mv = 0;
    working.data.hswing_fix = 0;
    switch (sw) {
      case climate::CLIMATE_SWING_BOTH:
        working.data.vswing_mv = 0x01;
        working.data.hswing_mv = 0x01;
        break;
      case climate::CLIMATE_SWING_VERTICAL:
        working.data.vswing_mv = 0x01;
        break;
      case climate::CLIMATE_SWING_HORIZONTAL:
        working.data.hswing_mv = 0x01;
        break;
      case climate::CLIMATE_SWING_OFF:
      default:
        break;
    }
  }

  this->build_set_cmd_(&working);
  this->pending_send_ = true;
}

// --- Heat/cool auto-switching logic ---

void TCLClimate::apply_heat_cool_logic_() {
  if (!this->heat_cool_mode_ || !this->got_first_response_)
    return;

  float temp = this->current_temperature;
  if (std::isnan(temp))
    return;

  auto &resp = this->last_resp_.data;
  float midpoint = (this->heat_cool_low_ + this->heat_cool_high_) / 2.0f;

  // Determine desired hardware mode
  uint8_t want_mode;
  float want_temp;
  if (temp < this->heat_cool_low_) {
    want_mode = 0x04;  // heat
    want_temp = this->heat_cool_low_;
  } else if (temp > this->heat_cool_high_) {
    want_mode = 0x01;  // cool
    want_temp = this->heat_cool_high_;
  } else {
    // In the deadband — keep current direction, or pick based on midpoint
    if (resp.power && (resp.mode == 0x01 || resp.mode == 0x04)) {
      // Already running heat or cool — let it ride
      return;
    }
    // Not running or in a different mode — pick based on side of midpoint
    if (temp <= midpoint) {
      want_mode = 0x04;  // heat
      want_temp = this->heat_cool_low_;
    } else {
      want_mode = 0x01;  // cool
      want_temp = this->heat_cool_high_;
    }
  }

  // Check if hardware already matches
  uint8_t want_temp_raw = (uint8_t) want_temp - 16;
  if (resp.power == 1 && resp.mode == want_mode && resp.temp == want_temp_raw)
    return;

  ESP_LOGD(TAG, "heat_cool: temp=%.1f low=%.0f high=%.0f -> %s @ %.0f",
           temp, this->heat_cool_low_, this->heat_cool_high_,
           want_mode == 0x04 ? "heat" : "cool", want_temp);

  GetResponse working{};
  memcpy(working.raw, this->last_resp_.raw, GET_RESP_LEN);
  working.data.power = 1;
  working.data.mode = want_mode;
  working.data.temp = want_temp_raw;

  this->build_set_cmd_(&working);
  this->pending_send_ = true;
}

// --- Swing control from select entities ---

void TCLClimate::set_vertical_swing(VerticalSwing vs) {
  if (!this->got_first_response_)
    return;

  GetResponse working{};
  memcpy(working.raw, this->last_resp_.raw, GET_RESP_LEN);

  working.data.vswing_mv = 0;
  working.data.vswing_fix = 0;

  switch (vs) {
    case VS_MOVE_FULL:  working.data.vswing_mv = 0x01; break;
    case VS_MOVE_UPPER: working.data.vswing_mv = 0x02; break;
    case VS_MOVE_LOWER: working.data.vswing_mv = 0x03; break;
    case VS_FIX_TOP:    working.data.vswing_fix = 0x01; break;
    case VS_FIX_UPPER:  working.data.vswing_fix = 0x02; break;
    case VS_FIX_MID:    working.data.vswing_fix = 0x03; break;
    case VS_FIX_LOWER:  working.data.vswing_fix = 0x04; break;
    case VS_FIX_BOTTOM: working.data.vswing_fix = 0x05; break;
    case VS_NONE:
    default:
      break;
  }

  if (working.data.vswing_mv)
    working.data.vswing = 1;
  else
    working.data.vswing = 0;

  this->build_set_cmd_(&working);
  this->pending_send_ = true;
}

void TCLClimate::set_horizontal_swing(HorizontalSwing hs) {
  if (!this->got_first_response_)
    return;

  GetResponse working{};
  memcpy(working.raw, this->last_resp_.raw, GET_RESP_LEN);

  working.data.hswing_mv = 0;
  working.data.hswing_fix = 0;

  switch (hs) {
    case HS_MOVE_FULL:     working.data.hswing_mv = 0x01; break;
    case HS_MOVE_LEFT:     working.data.hswing_mv = 0x02; break;
    case HS_MOVE_MID:      working.data.hswing_mv = 0x03; break;
    case HS_MOVE_RIGHT:    working.data.hswing_mv = 0x04; break;
    case HS_FIX_LEFT:      working.data.hswing_fix = 0x01; break;
    case HS_FIX_MID_LEFT:  working.data.hswing_fix = 0x02; break;
    case HS_FIX_MID:       working.data.hswing_fix = 0x03; break;
    case HS_FIX_MID_RIGHT: working.data.hswing_fix = 0x04; break;
    case HS_FIX_RIGHT:     working.data.hswing_fix = 0x05; break;
    case HS_NONE:
    default:
      break;
  }

  if (working.data.hswing_mv)
    working.data.hswing = 1;
  else
    working.data.hswing = 0;

  this->build_set_cmd_(&working);
  this->pending_send_ = true;
}

// --- Buzzer / Display ---

void TCLClimate::set_buzzer(bool on) {
  this->buzzer_on_ = on;
  if (!this->got_first_response_)
    return;
  GetResponse working{};
  memcpy(working.raw, this->last_resp_.raw, GET_RESP_LEN);
  this->build_set_cmd_(&working);
  this->pending_send_ = true;
}

void TCLClimate::set_display(bool on) {
  this->display_on_ = on;
  if (!this->got_first_response_)
    return;
  GetResponse working{};
  memcpy(working.raw, this->last_resp_.raw, GET_RESP_LEN);
  this->build_set_cmd_(&working);
  this->pending_send_ = true;
}

}  // namespace tcl_climate
}  // namespace esphome
