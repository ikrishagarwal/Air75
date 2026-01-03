#include QMK_KEYBOARD_H

#define NUM_LAYERS 4

enum custom_keycodes
{
  LYR_CYC = SAFE_RANGE,
  VIM_CPY,
  VIM_PST
};

const uint16_t PROGMEM keymaps[4][MATRIX_ROWS][MATRIX_COLS] = {
    /* BASE Layer 0 */
    [0] = LAYOUT(
        LYR_CYC, G(KC_L),
        KC_UP, KC_DOWN, KC_PGUP, KC_PGDN,
        KC_LEFT, KC_RGHT, KC_HOME, KC_END),

    /* Code Layer 1 */
    [1] = LAYOUT(
        _______, _______,
        KC_MPRV, KC_MPLY, KC_MNXT, LALT(KC_SPC),
        C(KC_C), C(KC_V), C(KC_X), C(KC_Z)),

    /* Vim Layer 2 */
    [2] = LAYOUT(
        _______, _______,
        KC_MPRV, KC_MPLY, KC_MNXT, LALT(KC_SPC),
        KC_J, KC_K, VIM_PST, VIM_CPY),

    /* Tools Layer 3 */
    [3] = LAYOUT(
        _______, _______,
        _______, _______, _______, _______,
        _______, _______, _______, _______)};

const uint16_t PROGMEM encoder_map[4][1][2] = {
    [0] = {ENCODER_CCW_CW(MS_WHLD, MS_WHLU)},
    [1] = {ENCODER_CCW_CW(KC_VOLU, KC_VOLD)},
    [2] = {ENCODER_CCW_CW(KC_VOLU, KC_VOLD)},
    [3] = {ENCODER_CCW_CW(KC_BRIU, KC_BRID)},
};

bool process_record_user(uint16_t keycode, keyrecord_t *record)
{
  if (record->event.pressed)
  {
    switch (keycode)
    {
    case LYR_CYC:
      uint8_t current_layer = get_highest_layer(layer_state);
      uint8_t next_layer = (current_layer + 1) % NUM_LAYERS;
      layer_move(next_layer);
      return false;

    case VIM_CPY:
      tap_code(KC_ESC);
      tap_code16(KC_DQUO);
      tap_code16(KC_ASTR);
      tap_code16(KC_Y);
      return false;

    case VIM_PST:
      tap_code(KC_ESC);
      tap_code16(KC_DQUO);
      tap_code16(KC_ASTR);
      tap_code16(KC_P);
      return false;
    }
  }
  return true;
}

oled_rotation_t oled_init_user(oled_rotation_t rotation)
{
  return OLED_ROTATION_180;
}

bool oled_task_user(void)
{
  static uint8_t last_layer = 255;
  uint8_t current_layer = get_highest_layer(layer_state);

  if (current_layer != last_layer)
  {
    oled_clear();
    oled_set_cursor(0, 0);
    oled_write_P(PSTR("Layer:"), false);

    switch (current_layer)
    {
    case 0:
      oled_write_P(PSTR("Base "), false);
      break;
    case 1:
      oled_write_P(PSTR("Code "), false);
      break;
    case 2:
      oled_write_P(PSTR("Vim  "), false);
      break;
    default:
      oled_write_P(PSTR("Tools"), false);
      break;
    }

    last_layer = current_layer;
  }

  return false;
}
