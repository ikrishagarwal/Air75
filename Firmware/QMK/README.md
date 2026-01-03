# Air75

![air75](https://raw.githubusercontent.com/ikrishagarwal/Air75/refs/heads/main/images/case.png)

A Macro Pad designed for productivity and efficiency, featuring a rotary encoder and an OLED display for enhanced functionality.

- Keyboard Maintainer: [Krish](https://github.com/ikrishagarwal)
- Hardware Supported: _Seeed Xiao RP2040_

Make example for this keyboard (after setting up your build environment):

```
make air75:via
```

Flashing example for this keyboard:

```
make air75:via:flash
```

See the [build environment setup](https://docs.qmk.fm/#/getting_started_build_tools) and the [make instructions](https://docs.qmk.fm/#/getting_started_make_guide) for more information. Brand new to QMK? Start with our [Complete Newbs Guide](https://docs.qmk.fm/#/newbs).

## Bootloader

Enter the bootloader in 3 ways:

- **Bootmagic reset**: Hold down the Rotary Encoder and plug in the keyboard
- **Physical reset button**: Hold down the `boot` button on the Xiao and plug in the keyboard
- **Keycode in layout**: Press the key mapped to `QK_BOOT` if it is available
