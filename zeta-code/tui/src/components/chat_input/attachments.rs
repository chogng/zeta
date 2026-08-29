//! Image attachment recognition, encoding, and chat_input placeholder bookkeeping.

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use std::path::PathBuf;
use zeta_utils_image::SupportedImageFormat;
use zeta_utils_image::data_url_from_bytes;
use zeta_utils_image::detect_image_format;

use super::editor::TextArea;
use super::editor::TextElementId;

const MAX_LOCAL_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Eq, PartialEq)]
struct AttachedImage {
    element_id: TextElementId,
    placeholder: String,
    data_url: String,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Attachments {
    images: Vec<AttachedImage>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum ImagePasteOutcome {
    Attached,
    NotImage,
    Rejected(String),
}

impl Attachments {
    pub(super) fn attach_image_bytes(
        &mut self,
        textarea: &mut TextArea,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        if bytes.len() as u64 > MAX_LOCAL_IMAGE_BYTES {
            return Err(format!(
                "pasted image exceeds the {} MiB limit",
                MAX_LOCAL_IMAGE_BYTES / 1024 / 1024
            ));
        }
        let format = detect_image_format(&bytes)
            .ok_or_else(|| "clipboard data is not a supported image".to_owned())?;
        self.insert_image(textarea, LoadedImage { format, bytes });
        Ok(())
    }

    pub(super) fn try_attach_pasted_path(
        &mut self,
        textarea: &mut TextArea,
        pasted: &str,
    ) -> ImagePasteOutcome {
        let Some(path) = normalize_pasted_path(pasted) else {
            return ImagePasteOutcome::NotImage;
        };
        let image = match load_image(&path) {
            Ok(Some(image)) => image,
            Ok(None) => return ImagePasteOutcome::NotImage,
            Err(error) => return ImagePasteOutcome::Rejected(error),
        };

        self.insert_image(textarea, image);
        ImagePasteOutcome::Attached
    }

    fn insert_image(&mut self, textarea: &mut TextArea, image: LoadedImage) {
        let placeholder = image_placeholder(self.images.len() + 1);
        let element_id = textarea.insert_element(&placeholder);
        textarea.insert_text(" ");
        self.images.push(AttachedImage {
            element_id,
            placeholder,
            data_url: data_url_from_bytes(image.format.mime_type(), &image.bytes),
        });
    }

    pub(super) fn reconcile(&mut self, textarea: &mut TextArea) {
        self.images
            .retain(|image| textarea.has_element(image.element_id));
        for (index, image) in self.images.iter_mut().enumerate() {
            let next_placeholder = image_placeholder(index + 1);
            if image.placeholder != next_placeholder {
                textarea.replace_element(image.element_id, &next_placeholder);
                image.placeholder = next_placeholder;
            }
        }
    }

    pub(super) fn image_url(&self, element_id: TextElementId) -> Option<&str> {
        self.images
            .iter()
            .find(|image| image.element_id == element_id)
            .map(|image| image.data_url.as_str())
    }

    pub(super) fn clear(&mut self) {
        self.images.clear();
    }
}

struct LoadedImage {
    format: SupportedImageFormat,
    bytes: Vec<u8>,
}

fn load_image(path: &Path) -> Result<Option<LoadedImage>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) if !has_supported_image_extension(path) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "could not read pasted image {}: {error}",
                path.display()
            ));
        }
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("could not inspect pasted image {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Ok(None);
    }

    let mut signature = [0; 12];
    let signature_len = file
        .read(&mut signature)
        .map_err(|error| format!("could not read pasted image {}: {error}", path.display()))?;
    if detect_image_format(&signature[..signature_len]).is_none() {
        return Ok(None);
    }
    if metadata.len() > MAX_LOCAL_IMAGE_BYTES {
        return Err(format!(
            "pasted image exceeds the {} MiB limit",
            MAX_LOCAL_IMAGE_BYTES / 1024 / 1024
        ));
    }

    file.rewind()
        .map_err(|error| format!("could not read pasted image {}: {error}", path.display()))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_LOCAL_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("could not read pasted image {}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_LOCAL_IMAGE_BYTES {
        return Err(format!(
            "pasted image exceeds the {} MiB limit",
            MAX_LOCAL_IMAGE_BYTES / 1024 / 1024
        ));
    }
    let Some(format) = detect_image_format(&bytes) else {
        return Ok(None);
    };
    Ok(Some(LoadedImage { format, bytes }))
}

fn normalize_pasted_path(pasted: &str) -> Option<PathBuf> {
    let pasted = pasted.trim();
    if pasted.is_empty() || pasted.contains(['\r', '\n']) {
        return None;
    }
    let unquoted = pasted
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            pasted
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(pasted);

    #[cfg(unix)]
    let unquoted = unescape_unix_path(unquoted);
    #[cfg(not(unix))]
    let unquoted = unquoted.to_owned();

    Some(PathBuf::from(unquoted))
}

#[cfg(unix)]
fn unescape_unix_path(path: &str) -> String {
    let mut unescaped = String::with_capacity(path.len());
    let mut chars = path.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            if let Some(escaped) = chars.next() {
                unescaped.push(escaped);
            }
        } else {
            unescaped.push(character);
        }
    }
    unescaped
}

fn has_supported_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp"
            )
        })
}

fn image_placeholder(number: usize) -> String {
    format!("[Image #{number}]")
}

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod tests;
