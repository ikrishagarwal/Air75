import board
import busio
import displayio
import terminalio
import i2cdisplaybus
import adafruit_displayio_ssd1306
from adafruit_display_text import label
import supervisor
import json

from kmk.kmk_keyboard import KMKKeyboard
from kmk.keys import KC
from kmk.scanners.keypad import MatrixScanner
from kmk.modules.layers import Layers
from kmk.modules.encoder import EncoderHandler
from kmk.extensions import Extension
from kmk.extensions.media_keys import MediaKeys

keyboard = KMKKeyboard()

# --- Hardware Configuration ---
# Matrix
keyboard.matrix = MatrixScanner(
    column_pins=[board.D0, board.D1, board.D2, board.D3],
    row_pins=[board.D8, board.D9, board.D10]
    # value_when_pressed=False, # removed because kmk gives error
)

# Rotary Encoder
# Rotary Encoder
encoder_handler = EncoderHandler()
encoder_handler.divisor = 4 # Reduce sensitivity/noise (STANDARD for EC11)
encoder_handler.pins = ((board.D6, board.D7, None, False),)
keyboard.modules.append(encoder_handler)

# --- Bongo Cat Bitmaps ---
# 32x22 pixels roughly for the cat. 
# We'll use a simple approach: 2 bitmaps for Up and Down states.
# Generated or Simulated Bitmap Data (1 = pixel on, 0 = off)

# Helper to create bitmap from ascii art or simple array would be nice, 
# but for compactness we use direct Bitmap logic or small shapes.
# Let's try drawing a simple graphical representation using displayio shapes if possible 
# OR just a small bitmap buffer.
# For simplicity and "fun", let's use a very low-res procedure to draw "Bongo Cat" style shapes.

def create_cat_bitmap(paws_down=False):
    # 32x32 area
    bmp = displayio.Bitmap(32, 32, 2) # Color depth 2 (0, 1)
    palette = displayio.Palette(2)
    palette[0] = 0x000000 # Black
    palette[1] = 0xFFFFFF # White
    
    # Draw Head (Simple Oval/Rect)
    for x in range(4, 28):
        for y in range(10, 25):
            bmp[x, y] = 1
            
    # Ears
    for x in range(4, 10):
        bmp[x, 9] = 1
        bmp[x+1, 8] = 1
    for x in range(22, 28):
        bmp[x, 9] = 1
        bmp[x-1, 8] = 1
        
    # Eyes
    bmp[10, 15] = 0
    bmp[11, 15] = 0
    bmp[20, 15] = 0
    bmp[21, 15] = 0
    
    # Table (Line)
    for x in range(0, 32):
        bmp[x, 25] = 1
        
    # Paws
    if paws_down:
        # Paws ON table (down)
        for x in range(8, 14): # Left Paw
            bmp[x, 26] = 1
            bmp[x, 27] = 1
        for x in range(18, 24): # Right Paw
            bmp[x, 26] = 1
            bmp[x, 27] = 1
    else:
        # Paws UP (in air)
        for x in range(6, 12): # Left Paw (raised)
            bmp[x, 18] = 1
            bmp[x, 19] = 1
        for x in range(20, 26): # Right Paw (raised)
            bmp[x, 18] = 1
            bmp[x, 19] = 1

    return bmp, palette

# --- Custom OLED Extension ---
class AdvancedOLED(Extension):
    def __init__(self, i2c):
        try:
            displayio.release_displays()
            self.display_bus = i2cdisplaybus.I2CDisplayBus(i2c, device_address=0x3C)
            self.display = adafruit_displayio_ssd1306.SSD1306(
                self.display_bus, width=128, height=32, rotation=180
            )
        except Exception as e:
            print(f"OLED Init Error: {e}")
            return

        self.splash = displayio.Group()
        self.display.root_group = self.splash

        # 1. Text Info Group (Left Side)
        self.info_group = displayio.Group()
        self.splash.append(self.info_group)
        
        self.line1 = label.Label(terminalio.FONT, text="LAYER: Code", color=0xFFFFFF, x=0, y=6)
        self.info_group.append(self.line1)
        
        self.line2 = label.Label(terminalio.FONT, text="VOL: ---", color=0xFFFFFF, x=0, y=24)
        self.info_group.append(self.line2)

        # 2. Bongo Cat Group (Right Side)
        self.cat_group = displayio.Group(x=96, y=0) # Right aligned
        self.splash.append(self.cat_group)
        
        # Create Frames
        self.bmp_up, self.pal = create_cat_bitmap(paws_down=False)
        self.bmp_down, _ = create_cat_bitmap(paws_down=True)
        
        self.cat_sprite = displayio.TileGrid(self.bmp_up, pixel_shader=self.pal)
        self.cat_group.append(self.cat_sprite)
        
        # State
        self.prev_layer = -1
        self.keys_down_count = 0
        self.serial_buffer = ""
        self.pc_media = "No Media"
        self.pc_mic = "LIVE"

    def update_text(self, keyboard, layer):
        layer_name = "Code" if layer == 0 else "Nav"
        self.line1.text = f"LAYER: {layer_name}"
        
        if layer == 0:
            # PC Stats on Layer 0 (Code)
            status = self.pc_mic if self.pc_mic else "???"
            media_short = self.pc_media[:10]
            self.line2.text = f"{status} | {media_short}"
        else:
            # Encoder Info on Layer 1 (Nav)
            self.line2.text = "Knob: VOL"

    def update_cat(self, typing):
        # Swap bitmap if typing
        if typing:
            self.cat_sprite.bitmap = self.bmp_down
        else:
            self.cat_sprite.bitmap = self.bmp_up

    def check_serial(self):
        # Read available bytes
        if supervisor.runtime.serial_bytes_available:
            try:
                data = sys.stdin.read(supervisor.runtime.serial_bytes_available) # Or use board specific
                # CircuitPython simple serial read via supervisor isn't direct stream often.
                # Actually, standard input is usually linked to USB CDC
                import sys
                # Read line if available?
                # Using a loop to drain buffer or read until newline
                # Simplified: assuming whole JSON packet comes in
                # We need to buffer valid json
                pass # Implementation detail: standard input reading in CP is tricky inside a loop without blocking.
                # Let's try to just read one char? No, use usb_cdc if available or sys.stdin
                # For safety in KMK loop, we skip blocking read.
            except:
                pass

    # Override for KMK main loop hook
    def after_matrix_scan(self, keyboard):
        # 1. Update Layer
        curr_layer = keyboard.active_layers[0] if keyboard.active_layers else 0
        if curr_layer != self.prev_layer:
            self.prev_layer = curr_layer
            self.update_text(keyboard, curr_layer)
        
        # 2. Update Cat Animation
        # Use matrix_update to detect activity
        # This usually contains the keys that changed state or are pressed
        is_typing = False
        if keyboard.matrix_update:
             is_typing = True
        
        # Optimization: Only update display if state changed
        if is_typing != self.keys_down_count: # reusing variable loosely as state flag
            self.keys_down_count = is_typing
            self.update_cat(is_typing)
        
        # 3. Read Serial (PC Data)
        # Rate Limit: Tuning to 100 iterations (more responsive but still limited)
        if not hasattr(self, '_serial_tick'): self._serial_tick = 0
        self._serial_tick = (self._serial_tick + 1) % 100
        
        if self._serial_tick == 0:
             if supervisor.runtime.serial_bytes_available:
                try:
                    # Limit read size
                    count = min(supervisor.runtime.serial_bytes_available, 128)
                    raw = sys.stdin.read(count)
                    if raw:
                        self.serial_buffer += raw
                        
                        # Buffer Overflow Protection
                        if len(self.serial_buffer) > 256:
                            self.serial_buffer = "" 
                            
                        if "\n" in self.serial_buffer:
                            lines = self.serial_buffer.split("\n")
                            # Process all complete lines
                            for line in lines[:-1]:
                                line = line.strip()
                                if line.startswith("{") and line.endswith("}"):
                                    try:
                                        data = json.loads(line)
                                        # print(f"DEBUG PARSE: {data}") # Debug logic
                                        self.pc_media = data.get("media", "No Media")
                                        self.pc_mic = data.get("mic", "LIVE")
                                        # Update text immediately if on Code layer
                                        if curr_layer == 0:
                                            self.update_text(keyboard, curr_layer)
                                    except Exception as e:
                                        print(f"JSON Err: {e}")
                            
                            self.serial_buffer = lines[-1] # Keep remainder
                except Exception as e:
                    print(f"Serial Err: {e}")

    def on_runtime_enable(self, sandbox): return
    def on_runtime_disable(self, sandbox): return
    def during_bootup(self, keyboard): return
    def before_matrix_scan(self, keyboard): return
    def before_hid_send(self, keyboard): return
    def after_hid_send(self, keyboard): return
    def on_powersave_enable(self, keyboard): return
    def on_powersave_disable(self, keyboard): return

# --- Main Setup ---
displayio.release_displays()
i2c_bus = busio.I2C(board.SCL, board.SDA)
oled_ext = AdvancedOLED(i2c_bus)
keyboard.extensions.append(oled_ext)

# Layers
layers_ext = Layers()
keyboard.modules.append(layers_ext)
keyboard.extensions.append(MediaKeys())

# --- Keymap (Same as before) ---
NO = KC.NO
TRNS = KC.TRNS
COPY = KC.LCTL(KC.C)
PASTE = KC.LCTL(KC.V)
CUT = KC.LCTL(KC.X)
UNDO = KC.LCTL(KC.Z)
TO_NAV = KC.TO(1)
TO_CODE = KC.TO(0)

keyboard.keymap = [
    [ # Code
        KC.MUTE, NO, NO, TO_NAV,
        COPY, PASTE, CUT, UNDO,
        KC.MPRV, KC.MPLY, KC.MNXT, KC.ENT,
    ],
    [ # Nav
        KC.MUTE, NO, NO, TO_CODE,
        KC.UP, KC.DOWN, KC.PGUP, KC.PGDN,
        KC.LEFT, KC.RIGHT, KC.UP, KC.DOWN,
    ]
]

encoder_handler.map = [
    ((KC.VOLU, KC.VOLD, KC.MUTE),),
    ((KC.VOLU, KC.VOLD, KC.MUTE),),
]

if __name__ == '__main__':
    keyboard.go()