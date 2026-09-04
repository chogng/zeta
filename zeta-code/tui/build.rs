//! Compiles the editable Welcome pet frames and actions into the TUI binary.

use std::env;
use std::fs;
use std::path::PathBuf;
use zeta_sprite::compile_sprite_sheet;
use zeta_sprite::terminal_sprite_sheet_rust_source;

const PET_SOURCE: &str = "assets/welcome/pet.sprite";
const PET_OUTPUT: &str = "welcome_pet.rs";
const PET_SOURCE_SIZE: (u32, u32) = (16, 16);
const PET_TERMINAL_SIZE: (u16, u16) = (8, 4);
const ALPHA_THRESHOLD: u8 = 128;

fn main() {
    println!("cargo:rerun-if-changed={PET_SOURCE}");

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must provide CARGO_MANIFEST_DIR"),
    );
    let source_path = manifest_dir.join(PET_SOURCE);
    let sheet = compile_sprite_sheet(&source_path, ALPHA_THRESHOLD)
        .unwrap_or_else(|error| panic!("compile {}: {error:#}", source_path.display()));
    assert_eq!(
        sheet.source_dimensions(),
        PET_SOURCE_SIZE,
        "{} must keep its 16x16 source canvas",
        source_path.display()
    );
    for frame in sheet.frames() {
        assert_eq!(
            (
                frame.sprite().as_sprite().width(),
                frame.sprite().as_sprite().height()
            ),
            PET_TERMINAL_SIZE,
            "{} frame '{}' must compile to 8x4 terminal cells",
            source_path.display(),
            frame.name()
        );
    }
    assert!(
        sheet.action("click").is_some(),
        "{} must define its click action",
        source_path.display()
    );
    let output_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR")).join(PET_OUTPUT);
    fs::write(
        &output_path,
        terminal_sprite_sheet_rust_source("PET", &sheet),
    )
    .unwrap_or_else(|error| panic!("write {}: {error}", output_path.display()));
}
