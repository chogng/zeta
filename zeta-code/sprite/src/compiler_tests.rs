use super::ansi_preview;
use super::rasterize;
use super::rust_source;
use super::source_dimensions;
use crate::pack_half_blocks_rgba;
use image::RgbaImage;
use std::fs;

#[test]
fn svg_png_and_sprite_sources_rasterize_to_the_requested_grid() {
    let directory = tempfile::tempdir().unwrap();
    let svg = directory.path().join("pet.svg");
    fs::write(
        &svg,
        r##"<svg width="4" height="4" xmlns="http://www.w3.org/2000/svg"><rect width="2" height="4" fill="#4085ac"/></svg>"##,
    )
    .unwrap();
    let png = directory.path().join("pet.png");
    RgbaImage::from_pixel(4, 4, image::Rgba([0x40, 0x85, 0xac, 0xff]))
        .save(&png)
        .unwrap();
    let sprite = directory.path().join("pet.sprite");
    fs::write(&sprite, "B=#4085AC\nK=#000000\n---\nBK\n.B\n").unwrap();

    assert_eq!(source_dimensions(&svg).unwrap(), (4, 4));
    assert_eq!(source_dimensions(&png).unwrap(), (4, 4));
    assert_eq!(source_dimensions(&sprite).unwrap(), (2, 2));
    for path in [&svg, &png, &sprite] {
        let raster = rasterize(path, 2, 2).unwrap();
        assert_eq!((raster.width, raster.height), (2, 2));
        assert_eq!(raster.pixels.len(), 4);
    }
    assert_eq!(
        rasterize(&sprite, 2, 2).unwrap().pixels,
        vec![
            [0x40, 0x85, 0xac, 0xff],
            [0x00, 0x00, 0x00, 0xff],
            [0x00, 0x00, 0x00, 0x00],
            [0x40, 0x85, 0xac, 0xff],
        ]
    );
}

#[test]
fn sprite_sources_reject_unknown_pixels_and_uneven_rows() {
    let directory = tempfile::tempdir().unwrap();
    let unknown = directory.path().join("unknown.sprite");
    fs::write(&unknown, "B=#4085AC\n---\nBX\n").unwrap();
    let uneven = directory.path().join("uneven.sprite");
    fs::write(&uneven, "B=#4085AC\n---\nBB\nB\n").unwrap();

    assert!(
        source_dimensions(&unknown)
            .unwrap_err()
            .to_string()
            .contains("undefined symbol 'X'")
    );
    assert!(
        source_dimensions(&uneven)
            .unwrap_err()
            .to_string()
            .contains("expected 2")
    );
}

#[test]
fn shrinking_pixel_art_preserves_symmetric_features() {
    let directory = tempfile::tempdir().unwrap();
    let svg = directory.path().join("symmetric.svg");
    fs::write(
        &svg,
        r##"<svg width="16" height="16" xmlns="http://www.w3.org/2000/svg">
            <rect x="2" width="2" height="4" fill="#4085ac"/>
            <rect x="12" width="2" height="4" fill="#4085ac"/>
            <rect x="2" y="4" width="12" height="10" fill="#4085ac"/>
            <rect x="4" y="6" width="2" height="3" fill="black"/>
            <rect x="10" y="6" width="2" height="3" fill="black"/>
        </svg>"##,
    )
    .unwrap();

    let raster = rasterize(&svg, 6, 6).unwrap();
    for row in raster.pixels.chunks_exact(6) {
        assert_eq!(row, row.iter().rev().copied().collect::<Vec<_>>());
    }
    assert!(raster.pixels.contains(&[0, 0, 0, 0xff]));
}

#[test]
fn previews_and_rust_assets_use_the_packed_cell_data() {
    let pixels = vec![[0x40, 0x85, 0xac, 0xff]; 4];
    let sprite = pack_half_blocks_rgba(2, 2, &pixels, 128).unwrap();

    let preview = ansi_preview(sprite.as_sprite());
    assert!(preview.contains("\x1b[38;2;64;133;172m█"));
    let source = rust_source("PET", &sprite);
    assert!(source.contains("pub(super) static PET: PetSprite = PetSprite::new("));
    assert!(source.contains("    2,"));
    assert!(source.contains("    1,"));
    assert!(source.contains("SpriteCell::new(\"█\", Some(Color::Rgb(0x40, 0x85, 0xac)), None)"));
    assert!(!source.contains("zeta_sprite"));
}

#[test]
fn two_color_rust_cells_are_emitted_in_rustfmt_ready_form() {
    let blue = [0x40, 0x85, 0xac, 0xff];
    let black = [0x00, 0x00, 0x00, 0xff];
    let sprite = pack_half_blocks_rgba(1, 2, &[blue, black], 128).unwrap();

    let source = rust_source("PET", &sprite);

    assert!(source.contains("        SpriteCell::new(\n"));
    assert!(source.contains("            Some(Color::Rgb(0x40, 0x85, 0xac)),\n"));
    assert!(source.contains("            Some(Color::Rgb(0x00, 0x00, 0x00)),\n"));
}
