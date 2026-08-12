//! Native system-clipboard text output and image input for the terminal host.

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ClipboardImage {
    pub(crate) png: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[cfg(not(target_os = "android"))]
pub(crate) fn write_text(text: &str) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("clipboard unavailable: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("could not write clipboard text: {error}"))
}

#[cfg(target_os = "android")]
pub(crate) fn write_text(_text: &str) -> Result<(), String> {
    Err("clipboard text output is unsupported on Android".into())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn read_image() -> Result<ClipboardImage, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("clipboard unavailable: {error}"))?;

    if let Some(image) = clipboard
        .get()
        .file_list()
        .unwrap_or_default()
        .into_iter()
        .find_map(|path| image::open(path).ok())
    {
        return encode_dynamic_image(image);
    }

    let image = clipboard
        .get_image()
        .map_err(|error| format!("no image on clipboard: {error}"))?;
    let width =
        u32::try_from(image.width).map_err(|_| "clipboard image width is too large".to_owned())?;
    let height = u32::try_from(image.height)
        .map_err(|_| "clipboard image height is too large".to_owned())?;
    let rgba = image.bytes.into_owned();
    let expected_length = image
        .width
        .checked_mul(image.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "clipboard image dimensions overflow".to_owned())?;
    if rgba.len() != expected_length {
        return Err("clipboard image contains an invalid RGBA buffer".into());
    }
    let rgba = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "clipboard image contains an invalid RGBA buffer".to_owned())?;

    encode_dynamic_image(image::DynamicImage::ImageRgba8(rgba))
}

#[cfg(target_os = "android")]
pub(crate) fn read_image() -> Result<ClipboardImage, String> {
    Err("clipboard image paste is unsupported on Android".into())
}

#[cfg(not(target_os = "android"))]
fn encode_dynamic_image(image: image::DynamicImage) -> Result<ClipboardImage, String> {
    let width = image.width();
    let height = image.height();
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|error| format!("could not encode clipboard image as PNG: {error}"))?;
    Ok(ClipboardImage { png, width, height })
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
