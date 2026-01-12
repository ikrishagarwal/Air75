#include QMK_KEYBOARD_H
#include "lib/bongocat.h"

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
        KC_MPRV, KC_MPLY, KC_MNXT, LALT(KC_SPC),
        KC_UP, KC_DOWN, KC_LEFT, KC_RGHT),

    /* Code Layer 1 */
    [1] = LAYOUT(
        _______, _______,
        _______, _______, _______, _______,
        C(KC_C), C(KC_V), C(KC_X), C(KC_Z)),

    /* Vim Layer 2 */
    [2] = LAYOUT(
        _______, _______,
        _______, _______, _______, _______,
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
    // Don't trigger the bongo animation for encoder rotations.
    // (These keycodes are emitted by `encoder_map`.)
    switch (keycode)
    {
    case MS_WHLD:
    case MS_WHLU:
    case KC_VOLU:
    case KC_VOLD:
    case KC_BRIU:
    case KC_BRID:
      break;
    default:
      bongocat_on_keypress();
      break;
    }

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

static const uint8_t *glyph_5x7(char c)
{
  // Minimal, custom 5x7 glyphs for: MOUSE / VOLUME / SCREEN
  // Bit 0 = top pixel, bit 6 = bottom pixel.
  static const uint8_t SPACE[5] = {0, 0, 0, 0, 0};
  static const uint8_t C_[5] = {0x1C, 0x22, 0x20, 0x22, 0x1C};
  static const uint8_t E_[5] = {0x3E, 0x2A, 0x2A, 0x2A, 0x22};
  static const uint8_t L_[5] = {0x3E, 0x20, 0x20, 0x20, 0x20};
  static const uint8_t M_[5] = {0x3F, 0x02, 0x04, 0x02, 0x3F};
  static const uint8_t N_[5] = {0x3E, 0x04, 0x08, 0x10, 0x3E};
  static const uint8_t O_[5] = {0x1C, 0x22, 0x22, 0x22, 0x1C};
  static const uint8_t R_[5] = {0x3E, 0x0A, 0x0A, 0x12, 0x24};
  static const uint8_t S_[5] = {0x24, 0x2A, 0x2A, 0x2A, 0x12};
  static const uint8_t U_[5] = {0x1E, 0x20, 0x20, 0x20, 0x1E};
  static const uint8_t V_[5] = {0x06, 0x18, 0x20, 0x18, 0x06};

  if (c >= 'a' && c <= 'z')
  {
    c = (char)(c - 'a' + 'A');
  }

  switch (c)
  {
  case 'C':
    return C_;
  case 'E':
    return E_;
  case 'L':
    return L_;
  case 'M':
    return M_;
  case 'N':
    return N_;
  case 'O':
    return O_;
  case 'R':
    return R_;
  case 'S':
    return S_;
  case 'U':
    return U_;
  case 'V':
    return V_;
  default:
    return SPACE;
  }
}

static void oled_write_text_5x7(uint8_t x, uint8_t y, const char *text)
{
  if (x >= OLED_DISPLAY_WIDTH || y >= OLED_DISPLAY_HEIGHT)
  {
    return;
  }

  const uint8_t page = y / 8;
  const uint8_t shift = y % 8;

  while (*text)
  {
    const uint8_t *glyph = glyph_5x7(*text++);
    for (uint8_t col = 0; col < 5; col++)
    {
      const uint8_t bits7 = glyph[col] & 0x7F;
      const uint8_t lo = (uint8_t)(bits7 << shift);
      const uint8_t hi = (shift == 0) ? 0 : (uint8_t)(bits7 >> (8 - shift));

      const uint8_t draw_x = (uint8_t)(x + col);
      if (draw_x < OLED_DISPLAY_WIDTH)
      {
        oled_write_raw_byte(lo, (uint16_t)page * OLED_DISPLAY_WIDTH + draw_x);
        if ((page + 1) < (OLED_DISPLAY_HEIGHT / 8))
        {
          oled_write_raw_byte(hi, (uint16_t)(page + 1) * OLED_DISPLAY_WIDTH + draw_x);
        }
      }
    }

    x = (uint8_t)(x + 6); // 5px glyph + 1px spacing
  }
}

static const char *get_encoder_purpose(uint8_t layer)
{
  switch (layer)
  {
  case 0:
    return "MOUSE";
  case 1:
  case 2:
    return "VOLUME";
  case 3:
    return "SCREEN";
  default:
    return "";
  }
}

bool oled_task_user(void)
{
  static bool bongocat_inited = false;
  static uint8_t last_layer = 255;
  uint8_t current_layer = get_highest_layer(layer_state);

  if (!bongocat_inited)
  {
    // Reserve the left side for text, draw the animation on the right.
    bongocat_set_x_offset(40);
    bongocat_inited = true;
  }

  if (current_layer != last_layer)
  {
    oled_clear();
    oled_set_cursor(0, 0);

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

    // Add ~1/3-line vertical spacing by drawing a small font at a custom Y.
    // QMK's built-in text rows are fixed 8px tall.
    // Row 0 text occupies y=0..7; start at y=10 to create a 2px gap (y=8..9).
    oled_write_text_5x7(0, 10, get_encoder_purpose(current_layer));

    last_layer = current_layer;
  }

  // Continuously render the animation (it updates based on timers + keypresses).
  render_bongocat();

  return false;
}
