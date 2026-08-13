use std::borrow::Cow;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ItemId;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillVersionSelector;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;
use zeta_protocol::UserInput;

use crate::CoreError;
use crate::image_preparation::prepare_user_image_data_url;

const MAX_REMOTE_IMAGE_URL_BYTES: usize = 8 * 1024;

pub(super) enum ValidatedUserInput<'a> {
    Text(&'a str),
    Image(Cow<'a, str>),
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
                Some(validate_image_url(url).map(ValidatedUserInput::Image))
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
                url: url.as_ref().to_owned(),
            },
        })
        .collect()
}

fn validate_image_url(url: &str) -> Result<Cow<'_, str>, CoreError> {
    if is_remote_image_url(url) {
        return Ok(Cow::Borrowed(url));
    }
    prepare_user_image_data_url(url)
        .map(Cow::Owned)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
}

fn is_remote_image_url(url: &str) -> bool {
    url.len() <= MAX_REMOTE_IMAGE_URL_BYTES
        && (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(char::is_whitespace)
}

#[cfg(test)]
#[path = "user_input_tests.rs"]
mod tests;
