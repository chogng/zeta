#[cfg(not(target_os = "android"))]
use super::*;

#[cfg(not(target_os = "android"))]
#[test]
fn rgba_clipboard_pixels_are_encoded_as_png() {
    let rgba = image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).unwrap();

    let encoded = encode_dynamic_image(image::DynamicImage::ImageRgba8(rgba)).unwrap();

    assert_eq!(encoded.width, 2);
    assert_eq!(encoded.height, 1);
    assert!(encoded.png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let decoded = image::load_from_memory(&encoded.png).unwrap().into_rgba8();
    assert_eq!(decoded.dimensions(), (2, 1));
    assert_eq!(decoded.into_raw(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
}
