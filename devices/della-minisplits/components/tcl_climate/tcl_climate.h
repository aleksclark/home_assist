#pragma once

#include "esphome/core/component.h"
#include "esphome/components/climate/climate.h"
#include "esphome/components/uart/uart.h"
#include "esphome/components/select/select.h"
#include "esphome/components/switch/switch.h"

namespace esphome {
namespace tcl_climate {

static const char *const TAG = "tcl_climate";

static constexpr uint8_t TCL_HEADER = 0xBB;
static constexpr int TCL_BAUD_RATE = 9600;
static constexpr int GET_RESP_LEN = 61;
static constexpr int SET_CMD_LEN = 35;
static constexpr int MAX_RX_LEN = 100;

static constexpr uint8_t REQ_CMD[] = {0xBB, 0x00, 0x01, 0x04, 0x02, 0x01, 0x00, 0xBD};
static constexpr uint8_t SET_CMD_BASE[SET_CMD_LEN] = {
    0xBB, 0x00, 0x01, 0x03, 0x1D, 0x00, 0x00, 0x64,
    0x03, 0xF3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00};

// --- Protocol bit-field structures (from OpenBeken drv_tclAC.h) ---

#pragma pack(push, 1)

union GetResponse {
  struct {
    uint8_t header;
    uint8_t byte_1;
    uint8_t byte_2;
    uint8_t type;
    uint8_t len;
    uint8_t byte_5;
    uint8_t byte_6;

    uint8_t mode : 4;
    uint8_t power : 1;
    uint8_t disp : 1;
    uint8_t eco : 1;
    uint8_t turbo : 1;

    uint8_t temp : 4;
    uint8_t fan : 3;
    uint8_t byte_8_bit_7 : 1;

    uint8_t byte_9;

    uint8_t byte_10_bit_0_4 : 5;
    uint8_t hswing : 1;
    uint8_t vswing : 1;
    uint8_t byte_10_bit_7 : 1;

    uint8_t byte_11;
    uint8_t byte_12;
    uint8_t byte_13;
    uint8_t byte_14;
    uint8_t byte_15;
    uint8_t byte_16;
    uint8_t byte_17;
    uint8_t byte_18;
    uint8_t byte_19;
    uint8_t byte_20;
    uint8_t byte_21;
    uint8_t byte_22;
    uint8_t byte_23;
    uint8_t byte_24;
    uint8_t byte_25;
    uint8_t byte_26;
    uint8_t byte_27;
    uint8_t byte_28;
    uint8_t byte_29;
    uint8_t byte_30;
    uint8_t byte_31;
    uint8_t byte_32;

    uint8_t byte_33_bit_0_6 : 7;
    uint8_t mute : 1;

    uint8_t byte_34;
    uint8_t byte_35;
    uint8_t byte_36;
    uint8_t byte_37;
    uint8_t byte_38;
    uint8_t byte_39;
    uint8_t byte_40;
    uint8_t byte_41;
    uint8_t byte_42;
    uint8_t byte_43;
    uint8_t byte_44;
    uint8_t byte_45;
    uint8_t byte_46;
    uint8_t byte_47;
    uint8_t byte_48;
    uint8_t byte_49;
    uint8_t byte_50;

    uint8_t vswing_fix : 3;
    uint8_t vswing_mv : 2;
    uint8_t byte_51_bit_5_7 : 3;

    uint8_t hswing_fix : 3;
    uint8_t hswing_mv : 3;
    uint8_t byte_52_bit_6_7 : 2;

    uint8_t byte_53;
    uint8_t byte_54;
    uint8_t byte_55;
    uint8_t byte_56;
    uint8_t byte_57;
    uint8_t byte_58;
    uint8_t byte_59;
    uint8_t xor_sum;
  } data;
  uint8_t raw[GET_RESP_LEN];
};

union SetCommand {
  struct {
    uint8_t header;
    uint8_t byte_1;
    uint8_t byte_2;
    uint8_t type;
    uint8_t len;
    uint8_t byte_5;
    uint8_t byte_6;

    uint8_t byte_7_bit_0_1 : 2;
    uint8_t power : 1;
    uint8_t off_timer_en : 1;
    uint8_t on_timer_en : 1;
    uint8_t beep : 1;
    uint8_t disp : 1;
    uint8_t eco : 1;

    uint8_t mode : 4;
    uint8_t byte_8_bit_4_5 : 2;
    uint8_t turbo : 1;
    uint8_t mute : 1;

    uint8_t temp : 4;
    uint8_t byte_9_bit_4_7 : 4;

    uint8_t fan : 3;
    uint8_t vswing : 3;
    uint8_t byte_10_bit_6 : 1;
    uint8_t byte_10_bit_7 : 1;

    uint8_t byte_11_bit_0_2 : 3;
    uint8_t hswing : 1;
    uint8_t byte_11_bit_4_7 : 4;

    uint8_t byte_12;
    uint8_t byte_13;

    uint8_t byte_14_bit_0_2 : 3;
    uint8_t byte_14_bit_3 : 1;
    uint8_t byte_14_bit_4 : 1;
    uint8_t half_degree : 1;
    uint8_t byte_14_bit_6_7 : 2;

    uint8_t byte_15;
    uint8_t byte_16;
    uint8_t byte_17;
    uint8_t byte_18;
    uint8_t byte_19;
    uint8_t byte_20;
    uint8_t byte_21;
    uint8_t byte_22;
    uint8_t byte_23;
    uint8_t byte_24;
    uint8_t byte_25;
    uint8_t byte_26;
    uint8_t byte_27;
    uint8_t byte_28;
    uint8_t byte_29;
    uint8_t byte_30;
    uint8_t byte_31;

    uint8_t vswing_fix : 3;
    uint8_t vswing_mv : 2;
    uint8_t byte_32_bit_5_7 : 3;

    uint8_t hswing_fix : 3;
    uint8_t hswing_mv : 3;
    uint8_t byte_33_bit_6_7 : 2;

    uint8_t xor_sum;
  } data;
  uint8_t raw[SET_CMD_LEN];
};

#pragma pack(pop)

// --- Vertical swing enum ---
enum VerticalSwing : uint8_t {
  VS_NONE = 0,
  VS_MOVE_FULL,
  VS_MOVE_UPPER,
  VS_MOVE_LOWER,
  VS_FIX_TOP,
  VS_FIX_UPPER,
  VS_FIX_MID,
  VS_FIX_LOWER,
  VS_FIX_BOTTOM,
};

static const char *const VSWING_NAMES[] = {
    "none", "move_full", "move_upper", "move_lower",
    "fix_top", "fix_upper", "fix_mid", "fix_lower", "fix_bottom"};

// --- Horizontal swing enum ---
enum HorizontalSwing : uint8_t {
  HS_NONE = 0,
  HS_MOVE_FULL,
  HS_MOVE_LEFT,
  HS_MOVE_MID,
  HS_MOVE_RIGHT,
  HS_FIX_LEFT,
  HS_FIX_MID_LEFT,
  HS_FIX_MID,
  HS_FIX_MID_RIGHT,
  HS_FIX_RIGHT,
};

static const char *const HSWING_NAMES[] = {
    "none", "move_full", "move_left", "move_mid", "move_right",
    "fix_left", "fix_mid_left", "fix_mid", "fix_mid_right", "fix_right"};

// --- Custom select for swing position ---
class TCLClimate;

class TCLSwingSelect : public select::Select, public Component {
 public:
  void set_parent(TCLClimate *parent) { this->parent_ = parent; }
  void set_is_vertical(bool is_vertical) { this->is_vertical_ = is_vertical; }

 protected:
  void control(const std::string &value) override;
  TCLClimate *parent_{nullptr};
  bool is_vertical_{true};
};

// --- Custom switch for buzzer/display ---
class TCLSwitch : public switch_::Switch, public Component {
 public:
  void set_parent(TCLClimate *parent) { this->parent_ = parent; }
  void set_is_buzzer(bool is_buzzer) { this->is_buzzer_ = is_buzzer; }

 protected:
  void write_state(bool state) override;
  TCLClimate *parent_{nullptr};
  bool is_buzzer_{true};
};

// --- Main climate component ---
class TCLClimate : public climate::Climate, public uart::UARTDevice, public PollingComponent {
 public:
  void setup() override;
  void loop() override;
  void update() override;
  void dump_config() override;
  float get_setup_priority() const override { return setup_priority::DATA; }

  void set_vertical_swing_select(TCLSwingSelect *sel) {
    this->vswing_select_ = sel;
    sel->set_parent(this);
    sel->set_is_vertical(true);
  }
  void set_horizontal_swing_select(TCLSwingSelect *sel) {
    this->hswing_select_ = sel;
    sel->set_parent(this);
    sel->set_is_vertical(false);
  }
  void set_buzzer_switch(TCLSwitch *sw) {
    this->buzzer_switch_ = sw;
    sw->set_parent(this);
    sw->set_is_buzzer(true);
  }
  void set_display_switch(TCLSwitch *sw) {
    this->display_switch_ = sw;
    sw->set_parent(this);
    sw->set_is_buzzer(false);
  }

  void set_vertical_swing(VerticalSwing vs);
  void set_horizontal_swing(HorizontalSwing hs);
  void set_buzzer(bool on);
  void set_display(bool on);

 protected:
  climate::ClimateTraits traits() override;
  void control(const climate::ClimateCall &call) override;

 private:
  void build_set_cmd_(GetResponse *resp);
  void send_set_cmd_();
  void send_poll_();
  void process_rx_();
  void parse_response_(uint8_t *buf, int len);

  // packet framing state machine
  int frame_byte_(int ch, uint8_t *buf, int buf_len);

  static uint8_t xor_checksum_(const uint8_t *buf, int len);
  static bool validate_xor_(const uint8_t *buf, int len);

  GetResponse last_resp_{};
  SetCommand set_cmd_{};
  bool pending_send_{false};
  bool got_first_response_{false};

  bool buzzer_on_{true};
  bool display_on_{true};

  VerticalSwing vswing_{VS_NONE};
  HorizontalSwing hswing_{HS_NONE};

  TCLSwingSelect *vswing_select_{nullptr};
  TCLSwingSelect *hswing_select_{nullptr};
  TCLSwitch *buzzer_switch_{nullptr};
  TCLSwitch *display_switch_{nullptr};

  // framing state
  int rx_pos_{0};
  bool rx_wait_len_{false};
  int rx_skip_{0};
  bool rx_logged_{false};
};

}  // namespace tcl_climate
}  // namespace esphome
