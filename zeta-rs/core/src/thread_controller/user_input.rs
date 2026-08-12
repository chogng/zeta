use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ItemId;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillVersionSelector;
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

pub(super) fn validate<'a>(
    input: &'a [UserInput],
    activated_skills: &[FrozenSkillActivation],
) -> Result<Vec<ValidatedUserInput<'a>>, CoreError> {
    if input.is_empty() {
        return Err(CoreError::InvalidInput(
            "Turn input must contain at least one item".into(),
        ));
    }

    let validated = input
        .iter()
        .filter_map(|input| match input {
            UserInput::Text { text } if !text.trim().is_empty() => {
                Some(Ok(ValidatedUserInput::Text(text)))
            }
            UserInput::Text { .. } => Some(Err(CoreError::InvalidInput(
                "Turn text input must not be empty".into(),
            ))),
            UserInput::Image { url } => {
                Some(validate_image_url(url).map(|()| ValidatedUserInput::Image(url)))
            }
            UserInput::Skill { .. } => None,
            UserInput::LocalImage { .. } | UserInput::Mention { .. } => {
                Some(Err(CoreError::InvalidInput(
                    "this Thread controller currently accepts text and normalized image URLs only"
                        .into(),
                )))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_skill_activations(input, activated_skills)?;
    if validated.is_empty() {
        return Err(CoreError::InvalidInput(
            "Turn input must include text or an image in addition to any Skill selection".into(),
        ));
    }
    Ok(validated)
}

fn validate_skill_activations(
    input: &[UserInput],
    activated_skills: &[FrozenSkillActivation],
) -> Result<(), CoreError> {
    let selected = input
        .iter()
        .filter_map(|input| match input {
            UserInput::Skill { skill } => Some(skill),
            UserInput::Text { .. }
            | UserInput::Image { .. }
            | UserInput::LocalImage { .. }
            | UserInput::Mention { .. } => None,
        })
        .collect::<Vec<_>>();
    let explicit = activated_skills
        .iter()
        .filter(|activation| activation.reason == SkillActivationReason::Explicit)
        .collect::<Vec<_>>();
    if selected.len() != explicit.len() {
        return Err(CoreError::InvalidInput(
            "every selected Skill must have one frozen activation".into(),
        ));
    }
    for (selected, activated) in selected.into_iter().zip(explicit) {
        if selected.id != activated.id {
            return Err(CoreError::InvalidInput(
                "frozen Skill activation does not match its explicit selection".into(),
            ));
        }
        if let SkillVersionSelector::PinnedDigest { digest } = &selected.version
            && digest != &activated.content_digest
        {
            return Err(CoreError::InvalidInput(
                "frozen Skill activation does not match its pinned digest".into(),
            ));
        }
    }
    Ok(())
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
