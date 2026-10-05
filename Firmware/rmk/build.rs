//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.
//!
//! The build script also sets the linker flags to tell it which link script to use.

use const_gen::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};
use xz2::read::XzEncoder;

fn main() {
    // Generate vial config at the root of project
    println!("cargo:rerun-if-changed=vial.json");
    println!("cargo:rerun-if-changed=keyboard.toml");

    generate_vial_config();
    generate_display_config();

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // Specify linker arguments.

    // `--nmagic` is required if memory section addresses are not aligned to 0x10000,
    // for example the FLASH and RAM sections in your `memory.x`.
    // See https://github.com/rust-embedded/cortex-m-quickstart/pull/95
    println!("cargo:rustc-link-arg=--nmagic");

    // Set the linker script to the one provided by cortex-m-rt.
    println!("cargo:rustc-link-arg=-Tlink.x");

    // Set the linker script of the defmt
    println!("cargo:rustc-link-arg=-Tdefmt.x");

    println!("cargo:rustc-linker=flip-link");
}

fn generate_display_config() {
    let content = fs::read_to_string("keyboard.toml").expect("Cannot read keyboard.toml");
    let declarations = display_config_declarations(&content)
        .unwrap_or_else(|error| panic!("Invalid display metadata in keyboard.toml: {error}"));
    let out_file = Path::new(&env::var_os("OUT_DIR").unwrap()).join("display_config_generated.rs");
    fs::write(out_file, declarations).expect("Cannot write display_config_generated.rs");
}

fn display_config_declarations(content: &str) -> Result<String, String> {
    let config: toml::Value =
        toml::from_str(content).map_err(|error| format!("Cannot parse TOML: {error}"))?;
    let layers = config
        .get("keymap")
        .and_then(|keymap| keymap.get("layer"))
        .and_then(toml::Value::as_array)
        .ok_or("Expected [[keymap.layer]] entries")?;
    let size = config
        .get("display")
        .and_then(|display| display.get("size"))
        .and_then(toml::Value::as_str)
        .ok_or("Expected [display].size as a WIDTHxHEIGHT string")?;
    let (width, height) = display_dimensions(size)?;

    let mut names = Vec::with_capacity(layers.len());
    let mut purposes = Vec::with_capacity(layers.len());
    for (index, layer) in layers.iter().enumerate() {
        let layer = layer
            .as_table()
            .ok_or_else(|| format!("keymap.layer[{index}] must be a table"))?;
        let name = match layer.get("name") {
            Some(name) => name
                .as_str()
                .ok_or_else(|| format!("keymap.layer[{index}].name must be a string"))?,
            None => "",
        };
        // Debug formatting produces a properly escaped Rust string literal,
        // including quotes, backslashes, newlines, and control characters.
        names.push(format!("{name:?}"));
        purposes.push(format!("{:?}", encoder_purpose(layer.get("encoders"))));
    }

    Ok(format!(
        "// Generated from keyboard.toml; do not edit.\n\
         pub const LAYER_NAMES: [&str; {count}] = [{names}];\n\
         pub const ENCODER_PURPOSES: [&str; {count}] = [{purposes}];\n\
         pub const DISPLAY_WIDTH: u32 = {width};\n\
         pub const DISPLAY_HEIGHT: u32 = {height};\n",
        count = layers.len(),
        names = names.join(", "),
        purposes = purposes.join(", "),
    ))
}

fn display_dimensions(size: &str) -> Result<(u32, u32), String> {
    let invalid = || {
        format!(
            "Invalid display size {size:?}; expected positive WIDTHxHEIGHT dimensions fitting i32"
        )
    };
    let (width, height) = size.split_once('x').ok_or_else(invalid)?;
    let parse = |dimension: &str| {
        if dimension.is_empty() || !dimension.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        dimension
            .parse::<u32>()
            // Drawing coordinates are signed, while embedded-graphics sizes are u32.
            .ok()
            .filter(|&value| value > 0 && value <= i32::MAX as u32)
            .ok_or_else(invalid)
    };
    Ok((parse(width)?, parse(height)?))
}

fn encoder_purpose(encoders: Option<&toml::Value>) -> &'static str {
    let Some(pair) = encoders
        .and_then(toml::Value::as_array)
        .and_then(|encoders| encoders.first())
        .and_then(toml::Value::as_array)
    else {
        return "";
    };
    if pair.len() != 2 {
        return "";
    }
    match (pair[0].as_str(), pair[1].as_str()) {
        (Some("MouseWheelDown"), Some("MouseWheelUp"))
        | (Some("MouseWheelUp"), Some("MouseWheelDown")) => "MOUSE",
        (Some("AudioVolDown"), Some("AudioVolUp")) | (Some("AudioVolUp"), Some("AudioVolDown")) => {
            "VOLUME"
        }
        _ => "",
    }
}

fn generate_vial_config() {
    // Generated vial config file
    let out_file = Path::new(&env::var_os("OUT_DIR").unwrap()).join("config_generated.rs");

    let p = Path::new("vial.json");
    let mut content = String::new();
    match File::open(p) {
        Ok(mut file) => {
            file.read_to_string(&mut content)
                .expect("Cannot read vial.json");
        }
        Err(e) => println!("Cannot find vial.json {:?}: {}", p, e),
    };

    let vial_cfg = json::stringify(json::parse(&content).unwrap());
    let mut keyboard_def_compressed: Vec<u8> = Vec::new();
    XzEncoder::new(vial_cfg.as_bytes(), 6)
        .read_to_end(&mut keyboard_def_compressed)
        .unwrap();

    let keyboard_id: Vec<u8> = vec![0xB9, 0xBC, 0x09, 0xB2, 0x9D, 0x37, 0x4C, 0xEA];
    let const_declarations = [
        const_declaration!(pub VIAL_KEYBOARD_DEF = keyboard_def_compressed),
        const_declaration!(pub VIAL_KEYBOARD_ID = keyboard_id),
    ]
    .join("\n");
    fs::write(out_file, const_declarations).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declarations(layers: &str) -> String {
        display_config_declarations(&format!("[display]\nsize = \"128x32\"\n{layers}"))
            .expect("valid metadata")
    }

    #[test]
    fn metadata_follows_actual_layers_and_defaults_missing_names() {
        let generated = declarations(
            r#"
[keymap]
layers = 99
[[keymap.layer]]
name = "General"
encoders = [["MouseWheelDown", "MouseWheelUp"]]
[[keymap.layer]]
encoders = [["AudioVolUp", "AudioVolDown"]]
[[keymap.layer]]
name = "Other"
"#,
        );
        assert!(generated
            .contains("pub const LAYER_NAMES: [&str; 3] = [\"General\", \"\", \"Other\"];"));
        assert!(generated
            .contains("pub const ENCODER_PURPOSES: [&str; 3] = [\"MOUSE\", \"VOLUME\", \"\"];"));
        assert!(generated.contains("pub const DISPLAY_WIDTH: u32 = 128;"));
        assert!(generated.contains("pub const DISPLAY_HEIGHT: u32 = 32;"));
    }

    #[test]
    fn names_are_escaped_as_rust_string_literals() {
        let generated = declarations(
            r#"
[[keymap.layer]]
name = "\"\\\n\t\r\u0000\u001B猫"
"#,
        );
        assert!(
            generated.contains(r#"pub const LAYER_NAMES: [&str; 1] = ["\"\\\n\t\r\0\u{1b}猫"];"#)
        );
    }

    #[test]
    fn exact_encoder_pairs_are_classified_in_either_order() {
        for (pair, expected) in [
            (r#"["MouseWheelDown", "MouseWheelUp"]"#, "MOUSE"),
            (r#"["MouseWheelUp", "MouseWheelDown"]"#, "MOUSE"),
            (r#"["AudioVolDown", "AudioVolUp"]"#, "VOLUME"),
            (r#"["AudioVolUp", "AudioVolDown"]"#, "VOLUME"),
        ] {
            let config: toml::Value = toml::from_str(&format!("encoders = [{pair}]")).unwrap();
            assert_eq!(encoder_purpose(config.get("encoders")), expected);
        }
    }

    #[test]
    fn other_or_missing_mappings_have_no_label_and_only_first_encoder_is_used() {
        assert_eq!(encoder_purpose(None), "");
        for encoders in [
            "[]",
            "[[]]",
            r#"[["MouseWheelDown"]]"#,
            r#"[["MouseWheelDown", "MouseWheelUp", "No"]]"#,
            r#"[["MouseWheelDown", "MouseWheelDown"]]"#,
            r#"[["MouseWheelDown", "AudioVolUp"]]"#,
            r#"[["WM(MouseWheelDown, LCtrl)", "MouseWheelUp"]]"#,
            r#"[["No", "No"], ["MouseWheelDown", "MouseWheelUp"]]"#,
            r#"["MouseWheelDown", "MouseWheelUp"]"#,
            "[[1, 2]]",
            "false",
        ] {
            let config: toml::Value = toml::from_str(&format!("encoders = {encoders}")).unwrap();
            assert_eq!(encoder_purpose(config.get("encoders")), "", "{encoders}");
        }
        let config: toml::Value = toml::from_str(
            "encoders = [[\"MouseWheelUp\", \"MouseWheelDown\"], [\"AudioVolUp\", \"AudioVolDown\"]]",
        )
        .unwrap();
        assert_eq!(encoder_purpose(config.get("encoders")), "MOUSE");
    }

    #[test]
    fn dimensions_are_positive_and_safe_for_drawing_coordinates() {
        assert_eq!(display_dimensions("128x32"), Ok((128, 32)));
        assert_eq!(display_dimensions("1x1"), Ok((1, 1)));
        assert_eq!(display_dimensions("2147483647x1"), Ok((i32::MAX as u32, 1)));
        for size in [
            "",
            "128",
            "128X32",
            "128x32x1",
            "0x32",
            "128x0",
            "-1x32",
            "128x-1",
            "1.5x32",
            "x32",
            "128x",
            "+1x32",
            " 128x32",
            "128x32 ",
            "2147483648x1",
            "1x2147483648",
            "4294967296x32",
        ] {
            assert!(display_dimensions(size).is_err(), "accepted {size:?}");
        }
    }

    #[test]
    fn invalid_toml_or_missing_required_metadata_is_rejected() {
        for config in [
            "[broken",
            "[[keymap.layer]]\nname = \"No display\"",
            "[display]\nsize = \"128x32\"",
            "[display]\nsize = 128\n[[keymap.layer]]",
            "[display]\nsize = \"0x32\"\n[[keymap.layer]]",
            "[display]\nsize = \"128x32\"\n[[keymap.layer]]\nname = 1",
            "[display]\nsize = \"128x32\"\n[keymap]\nlayer = [1]",
        ] {
            assert!(
                display_config_declarations(config).is_err(),
                "accepted {config:?}"
            );
        }
    }
}
