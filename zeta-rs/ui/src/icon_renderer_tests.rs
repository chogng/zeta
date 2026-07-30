use zeta_icons::{Icon, IconDefinition, IconId, icons};

use super::{ATLAS_PADDING, ATLAS_SIZE, ShelfAllocator, rasterize_icon, validate_paint_icon};
use crate::{Color, PaintIcon, Rect, UiRenderError};

const TEST_ICON: Icon = Icon::new(
    IconId::new("circle"),
    IconDefinition::symbolic(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6"/></svg>"#,
    ),
);

#[test]
fn rasterizes_svg_as_alpha_mask_at_requested_physical_size() {
    let mask = rasterize_icon(TEST_ICON, 32, 24).unwrap();

    assert_eq!(mask.len(), 32 * 24);
    assert_eq!(mask[0], 0);
    assert!(mask.contains(&255));
}

#[test]
fn shelf_allocator_keeps_padding_between_regions() {
    let mut allocator = ShelfAllocator::new();
    let first = allocator.allocate(16, 16).unwrap();
    let second = allocator.allocate(24, 16).unwrap();

    assert_eq!(first.x, ATLAS_PADDING);
    assert_eq!(first.y, ATLAS_PADDING);
    assert_eq!(second.x, first.x + first.width + ATLAS_PADDING);
    assert_eq!(second.y, first.y);
}

#[test]
fn shelf_allocator_rejects_icon_larger_than_atlas() {
    let mut allocator = ShelfAllocator::new();

    assert!(allocator.allocate(ATLAS_SIZE, 16).is_none());
}

#[test]
fn rejects_negative_icon_bounds() {
    let icon = PaintIcon::new(
        TEST_ICON,
        Rect::from_xywh(0.0, 0.0, -16.0, 16.0),
        Color::WHITE,
    );

    assert!(matches!(
        validate_paint_icon(2, icon),
        Err(UiRenderError::InvalidPaintIcon {
            index: 2,
            reason: "bounds must not be negative",
        })
    ));
}

#[test]
fn reports_invalid_svg_with_icon_name() {
    let icon = Icon::new(IconId::new("broken"), IconDefinition::symbolic(b"<svg"));

    assert!(matches!(
        rasterize_icon(icon, 16, 16),
        Err(UiRenderError::InvalidSvgIcon { name: "broken", .. })
    ));
}

#[test]
fn rejects_multicolor_artwork_before_symbolic_rasterization() {
    assert!(matches!(
        rasterize_icon(icons::LAYOUT_PANEL_OFF, 16, 16),
        Err(UiRenderError::UnsupportedMulticolorIcon {
            name: "layout-panel-off"
        })
    ));
}
