//! Compiles the editable Welcome pet into the TUI binary.

use std::env;
use std::fs;
use std::path::PathBuf;
use zeta_sprite::compile_sprite_grid;
use zeta_sprite::terminal_sprite_rust_source;

const PET_SOURCE: &str = "assets/welcome/pet.sprite";
const PET_OUTPUT: &str = "welcome_pet.rs";
const ALPHA_THRESHOLD: u8 = 128;

fn main() {
    println!("cargo:rerun-if-changed={PET_SOURCE}");

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must provide CARGO_MANIFEST_DIR"),
    );
    let source_path = manifest_dir.join(PET_SOURCE);
    let sprite = compile_sprite_grid(&source_path, ALPHA_THRESHOLD)
        .unwrap_or_else(|error| panic!("compile {}: {error:#}", source_path.display()));
    let output_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR")).join(PET_OUTPUT);
    fs::write(&output_path, terminal_sprite_rust_source("PET", &sprite))
        .unwrap_or_else(|error| panic!("write {}: {error}", output_path.display()));
}
