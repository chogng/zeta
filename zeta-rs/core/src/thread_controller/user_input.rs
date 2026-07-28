use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use zeta_protocol::ItemId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;
use zeta_protocol::UserInput;

use crate::CoreError;

const MAX_LOCAL_IMAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_REMOTE_IMAGE_URL_BYTES: usize = 8 * 1024;
const MAX_ENCODED_IMAGE_BYTES: usize = MAX_LOCAL_IMAGE_BYTES.div_ceil(3) * 4;

pub(super) enum ValidatedUserInput<'a> {
    Text(&'a str),
    Image(&'a str),
}

pub(super) fn validate(input: &[UserInput]) -> Result<Vec<ValidatedUserInput<'_>>, CoreError> {
    if input.is_empty() {
        return Err(CoreError::InvalidInput(
            "Turn input must contain at least one item".into(),
        ));
    }

    input
        .iter()
        .map(|input| match input {
            UserInput::Text { text } if !text.trim().is_empty() => {
                Ok(ValidatedUserInput::Text(text))
            }
            UserInput::Text { .. } => Err(CoreError::InvalidInput(
                "Turn text input must not be empty".into(),
            )),
            UserInput::Image { url } => {
                validate_image_url(url)?;
                Ok(ValidatedUserInput::Image(url))
            }
            UserInput::LocalImage { .. } | UserInput::Skill { .. } | UserInput::Mention { .. } => {
                Err(CoreError::InvalidInput(
                    "this Thread controller currently accepts text and normalized image URLs only"
                        .into(),
                ))
            }
        })
        .collect()
}

pub(super) fn thread_items(
    input: &[ValidatedUserInput<'_>],
    turn_id: &TurnId,
    mut next_item_id: impl FnMut() -> ItemId,
) -> Vec<ThreadItem> {
    input
        .iter()
        .map(|input| match input {
            ValidatedUserInput::Text(text) => ThreadItem::UserMessage {
                item_id: next_item_id(),
                turn_id: turn_id.clone(),
                text: (*text).to_owned(),
            },
            ValidatedUserInput::Image(url) => ThreadItem::UserImage {
                item_id: next_item_id(),
                turn_id: turn_id.clone(),
                url: (*url).to_owned(),
            },
        })
        .collect()
}

fn validate_image_url(url: &str) -> Result<(), CoreError> {
    if is_remote_image_url(url) {
        return Ok(());
    }

    let Some((header, encoded)) = url.split_once(',') else {
        return Err(invalid_image_url());
    };
    let mime_type = header
        .strip_prefix("data:")
        .and_then(|header| header.strip_suffix(";base64"))
        .filter(|mime_type| is_supported_mime_type(mime_type))
        .ok_or_else(invalid_image_url)?;
    if encoded.is_empty() || encoded.len() > MAX_ENCODED_IMAGE_BYTES {
        return Err(CoreError::InvalidInput(format!(
            "image input exceeds the {MAX_LOCAL_IMAGE_BYTES} byte limit"
        )));
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| CoreError::InvalidInput("image input contains invalid base64".into()))?;
    if bytes.len() > MAX_LOCAL_IMAGE_BYTES {
        return Err(CoreError::InvalidInput(format!(
            "image input exceeds the {MAX_LOCAL_IMAGE_BYTES} byte limit"
        )));
    }
    if !signature_matches(mime_type, &bytes) {
        return Err(CoreError::InvalidInput(
            "image MIME type does not match its encoded content".into(),
        ));
    }
    Ok(())
}

fn is_remote_image_url(url: &str) -> bool {
    url.len() <= MAX_REMOTE_IMAGE_URL_BYTES
        && (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(char::is_whitespace)
}

fn is_supported_mime_type(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}

fn signature_matches(mime_type: &str, bytes: &[u8]) -> bool {
    match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(b"\xff\xd8\xff"),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        _ => false,
    }
}

fn invalid_image_url() -> CoreError {
    CoreError::InvalidInput(
        "image input must be an HTTP(S) URL or a supported base64 data URL".into(),
    )
}

#[cfg(test)]
#[path = "user_input_tests.rs"]
mod tests;
