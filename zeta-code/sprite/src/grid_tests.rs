use super::compile_sprite_grid;
use std::fs;

#[test]
fn exact_grid_compilation_preserves_an_odd_source_edge() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pet.sprite");
    fs::write(&path, "B=#4085AC\n---\nBB\nBB\n.B\n").unwrap();

    let sprite = compile_sprite_grid(&path, 128).unwrap();

    assert_eq!(sprite.as_sprite().width(), 1);
    assert_eq!(sprite.as_sprite().height(), 2);
    assert_eq!(sprite.as_sprite().cells()[0].symbol(), '█');
    assert_eq!(sprite.as_sprite().cells()[1].symbol(), '▝');
}

#[test]
fn exact_grid_compilation_rejects_invalid_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pet.sprite");
    fs::write(&path, "B=#4085AC\n---\nBX\n").unwrap();

    let error = compile_sprite_grid(&path, 128).unwrap_err();

    assert!(error.to_string().contains("undefined symbol 'X'"));
}
