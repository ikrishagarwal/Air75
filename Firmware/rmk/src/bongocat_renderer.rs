use embassy_time::{Duration, Instant};
use embedded_graphics::{
    mono_font::{ascii::FONT_5X8, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use rmk::{
    display::{DisplayRenderer, RenderContext},
    event::{KeyboardEvent, KeyboardEventPos, SubscribableEvent},
};

use crate::bongocat_frames::{FRAME_HEIGHT, FRAME_WIDTH, IDLE_FRAMES, TAP_FRAMES};

const IDLE_FRAME_DURATION: Duration = Duration::from_millis(300);
const TAP_TIMEOUT: Duration = Duration::from_millis(220);

// Layer labels, encoder labels, and dimensions share keyboard.toml's defaults.
include!(concat!(env!("OUT_DIR"), "/display_config_generated.rs"));

const X_OFFSET: usize = 40;
const TARGET_WIDTH: usize = (DISPLAY_WIDTH as usize).saturating_sub(X_OFFSET);

const TEXT_STYLE: MonoTextStyle<'static, BinaryColor> =
    MonoTextStyle::new(&FONT_5X8, BinaryColor::On);

// These glyphs match the custom 5x7 font used by the QMK implementation. The
// bit at position zero is the top pixel of each glyph column.
const SPACE_GLYPH: [u8; 5] = [0, 0, 0, 0, 0];
const C_GLYPH: [u8; 5] = [0x1C, 0x22, 0x20, 0x22, 0x1C];
const E_GLYPH: [u8; 5] = [0x3E, 0x2A, 0x2A, 0x2A, 0x22];
const L_GLYPH: [u8; 5] = [0x3E, 0x20, 0x20, 0x20, 0x20];
const M_GLYPH: [u8; 5] = [0x3F, 0x02, 0x04, 0x02, 0x3F];
const N_GLYPH: [u8; 5] = [0x3E, 0x04, 0x08, 0x10, 0x3E];
const O_GLYPH: [u8; 5] = [0x1C, 0x22, 0x22, 0x22, 0x1C];
const R_GLYPH: [u8; 5] = [0x3E, 0x0A, 0x0A, 0x12, 0x24];
const S_GLYPH: [u8; 5] = [0x24, 0x2A, 0x2A, 0x2A, 0x12];
const U_GLYPH: [u8; 5] = [0x1E, 0x20, 0x20, 0x20, 0x1E];
const V_GLYPH: [u8; 5] = [0x06, 0x18, 0x20, 0x18, 0x06];

fn glyph_5x7(c: char) -> &'static [u8; 5] {
    match c {
        'C' => &C_GLYPH,
        'E' => &E_GLYPH,
        'L' => &L_GLYPH,
        'M' => &M_GLYPH,
        'N' => &N_GLYPH,
        'O' => &O_GLYPH,
        'R' => &R_GLYPH,
        'S' => &S_GLYPH,
        'U' => &U_GLYPH,
        'V' => &V_GLYPH,
        _ => &SPACE_GLYPH,
    }
}

pub struct BongocatRenderer {
    key_sub: <KeyboardEvent as SubscribableEvent>::Subscriber,
    idle_frame: usize,
    tap_frame: usize,
    last_keypress: Option<Instant>,
    last_animation_time: Instant,
}

impl Default for BongocatRenderer {
    fn default() -> Self {
        Self {
            key_sub: KeyboardEvent::subscriber(),
            idle_frame: 0,
            tap_frame: 0,
            last_keypress: None,
            last_animation_time: Instant::now(),
        }
    }
}

impl BongocatRenderer {
    fn is_tapping(&self) -> bool {
        self.last_keypress
            .is_some_and(|t| Instant::now().duration_since(t) < TAP_TIMEOUT)
    }

    fn current_frame(&self) -> &'static [u8] {
        if self.is_tapping() {
            // QMK reverses the tap-frame order when it writes the frame.
            &TAP_FRAMES[TAP_FRAMES.len() - 1 - self.tap_frame]
        } else {
            // Keep the same idle sequence as QMK: 3, 2, 1, 0, 4, ...
            &IDLE_FRAMES[IDLE_FRAMES.len() - 1 - self.idle_frame]
        }
    }

    fn advance_animation(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_animation_time);

        // Tap frames change on keypresses, not on periodic redraws.
        if !self.is_tapping() && elapsed >= IDLE_FRAME_DURATION {
            self.idle_frame = (self.idle_frame + 1) % IDLE_FRAMES.len();
            self.last_animation_time = now;
        }
    }

    fn draw_encoder_purpose<D: DrawTarget<Color = BinaryColor>>(display: &mut D, purpose: &str) {
        for (char_index, c) in purpose.chars().enumerate() {
            let glyph = glyph_5x7(c);
            let x = char_index * 6;
            for (column, bits) in glyph.iter().enumerate() {
                for row in 0..7 {
                    if bits & (1 << row) != 0 {
                        Pixel(Point::new((x + column) as i32, 10 + row), BinaryColor::On)
                            .draw(display)
                            .ok();
                    }
                }
            }
        }
    }

    fn draw_frame<D: DrawTarget<Color = BinaryColor>>(display: &mut D, frame: &[u8]) {
        // QMK stores these frames in SSD1306 page format: 128 columns per
        // page, with bit 0 at the top of each column. Since the bongo cat is
        // drawn on the right side, use the same full-frame horizontal scaling
        // as bongocat_write_frame_P instead of cropping the left 40 columns.
        for dest_col in 0..TARGET_WIDTH {
            let src_col = if TARGET_WIDTH > 1 {
                dest_col * (FRAME_WIDTH - 1) / (TARGET_WIDTH - 1)
            } else {
                0
            };

            for row in 0..DISPLAY_HEIGHT as usize {
                let src_row = row * FRAME_HEIGHT / DISPLAY_HEIGHT as usize;
                let source_byte = frame[(src_row / 8) * FRAME_WIDTH + src_col];
                if source_byte & (1 << (src_row % 8)) != 0 {
                    Pixel(
                        Point::new((X_OFFSET + dest_col) as i32, row as i32),
                        BinaryColor::On,
                    )
                    .draw(display)
                    .ok();
                }
            }
        }
    }
}

impl DisplayRenderer<BinaryColor> for BongocatRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        // Consume individual events rather than key_press_latch: the latter
        // also includes encoder presses and coalesces multiple keypresses.
        while let Some(event) = self.key_sub.try_next_message_pure() {
            if event.pressed && matches!(event.pos, KeyboardEventPos::Key(_)) {
                let now = Instant::now();
                self.last_keypress = Some(now);
                self.tap_frame = (self.tap_frame + 1) % TAP_FRAMES.len();
                self.last_animation_time = now;
            }
        }
        self.advance_animation();
        display.clear(BinaryColor::Off).ok();

        let layer_idx = usize::from(ctx.layer);
        if layer_idx < LAYER_NAMES.len() {
            Text::with_baseline(
                LAYER_NAMES[layer_idx],
                Point::new(0, 0),
                TEXT_STYLE,
                Baseline::Top,
            )
            .draw(display)
            .ok();
        }

        if layer_idx < ENCODER_PURPOSES.len() {
            Self::draw_encoder_purpose(display, ENCODER_PURPOSES[layer_idx]);
        }

        Self::draw_frame(display, self.current_frame());
    }
}
