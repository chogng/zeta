use super::PhysicalExtent;

#[test]
fn physical_extent_preserves_platform_pixel_dimensions() {
    let extent = PhysicalExtent::new(1440, 900);

    assert_eq!(extent.width, 1440);
    assert_eq!(extent.height, 900);
}
