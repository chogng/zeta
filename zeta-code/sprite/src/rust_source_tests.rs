use super::terminal_sprite_rust_source;
use crate::pack_half_blocks_rgba;

#[test]
fn rust_assets_use_the_packed_cell_data() {
    let pixels = vec![[0x40, 0x85, 0xac, 0xff]; 4];
    let sprite = pack_half_blocks_rgba(2, 2, &pixels, 128).unwrap();

    let source = terminal_sprite_rust_source("PET", &sprite);

    assert!(
        source.contains("pub(super) static PET: TerminalSprite<'static> = TerminalSprite::new(")
    );
    assert!(source.contains("    2,"));
    assert!(source.contains("    1,"));
    assert!(source.contains("SpriteCell::new('█', Some(Rgb::new(0x40, 0x85, 0xac)), None)"));
    assert!(source.contains("use zeta_sprite::TerminalSprite;"));
}

#[test]
fn two_color_rust_cells_are_emitted_in_rustfmt_ready_form() {
    let blue = [0x40, 0x85, 0xac, 0xff];
    let black = [0x00, 0x00, 0x00, 0xff];
    let sprite = pack_half_blocks_rgba(1, 2, &[blue, black], 128).unwrap();

    let source = terminal_sprite_rust_source("PET", &sprite);

    assert!(source.contains("        SpriteCell::new(\n"));
    assert!(source.contains("            Some(Rgb::new(0x40, 0x85, 0xac)),\n"));
    assert!(source.contains("            Some(Rgb::new(0x00, 0x00, 0x00)),\n"));
}
