use super::Args;
use super::check_rust_output;
use super::constant_name;
use super::raster_dimensions;
use super::terminal_dimensions;
use clap::Parser;
use std::fs;
use std::path::Path;

#[test]
fn default_terminal_dimensions_compensate_for_tall_cells() {
    assert_eq!(terminal_dimensions(16, 16, None, None).unwrap(), (16, 8));
    assert_eq!(
        terminal_dimensions(32, 16, Some(12), None).unwrap(),
        (12, 3)
    );
    assert_eq!(terminal_dimensions(32, 16, None, Some(3)).unwrap(), (12, 3));
}

#[test]
fn matching_pixel_grid_preserves_an_odd_source_edge() {
    assert_eq!(
        raster_dimensions(Path::new("pet.sprite"), 16, 9, 8, 5).unwrap(),
        (16, 9)
    );
    assert_eq!(
        raster_dimensions(Path::new("pet.svg"), 16, 9, 8, 5).unwrap(),
        (16, 10)
    );
    assert_eq!(
        raster_dimensions(Path::new("pet.sprite"), 16, 9, 8, 4).unwrap(),
        (16, 8)
    );
}

#[test]
fn rust_constant_names_are_explicit_and_stable() {
    assert_eq!(constant_name("WELCOME_PET").unwrap(), "WELCOME_PET");
    assert!(constant_name("WelcomePet").is_err());
    assert!(constant_name("9PET").is_err());
}

#[test]
fn check_requires_a_rust_output_path() {
    assert!(Args::try_parse_from(["zeta-sprite", "pet.svg", "--check"]).is_err());
}

#[test]
fn rust_output_check_rejects_stale_generated_source() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("asset.rs");
    fs::write(&output, "expected").unwrap();

    check_rust_output(&output, "expected").unwrap();
    let error = check_rust_output(&output, "changed").unwrap_err();

    assert!(error.to_string().contains("is out of date"));
}
