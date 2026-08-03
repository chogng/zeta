use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use zeta_app_server_client::local_profile_root;
use zeta_keybinding::{
    ContextExpression, HostPlatform, format_key_sequence, parse_key_sequence,
    serialize_key_sequence,
};
use zeta_utils_path::write_atomically;

use crate::commands::NativeCommand;
use crate::keybindings::{
    NativeBindingCondition, NativeKeybindingContext, NativeKeybindings, NativeUserBinding,
    NativeUserBindingTarget,
};

const MAX_RESOURCE_BYTES: u64 = 1024 * 1024;
const MAX_BINDINGS: usize = 1_024;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSnapshot {
    Missing,
    Contents(Vec<u8>),
}

pub(crate) struct KeybindingsResource {
    path: PathBuf,
    platform: HostPlatform,
    observed: Option<ResourceSnapshot>,
    diagnostics: Vec<String>,
    next_poll: Instant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeybindingsResourcePoll {
    Unchanged,
    Updated,
    Rejected(String),
}

impl KeybindingsResource {
    pub(crate) fn for_workspace(_workspace_root: &Path, platform: HostPlatform) -> Self {
        let state_root = local_profile_root();
        Self::new(
            state_root.join("keybindings.json"),
            platform,
            Instant::now(),
        )
    }

    fn new(path: PathBuf, platform: HostPlatform, now: Instant) -> Self {
        Self {
            path,
            platform,
            observed: None,
            diagnostics: Vec::new(),
            next_poll: now,
        }
    }

    pub(crate) fn poll(
        &mut self,
        now: Instant,
        keybindings: &mut NativeKeybindings,
    ) -> KeybindingsResourcePoll {
        if now < self.next_poll {
            return KeybindingsResourcePoll::Unchanged;
        }
        self.next_poll = now + POLL_INTERVAL;
        let snapshot = match read_snapshot(&self.path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let diagnostic = format!("could not read {}: {error}", self.path.display());
                self.diagnostics = vec![diagnostic.clone()];
                return KeybindingsResourcePoll::Rejected(diagnostic);
            }
        };
        if self.observed.as_ref() == Some(&snapshot) {
            return KeybindingsResourcePoll::Unchanged;
        }
        self.observed = Some(snapshot.clone());
        let rules = match snapshot {
            ResourceSnapshot::Missing => Vec::new(),
            ResourceSnapshot::Contents(contents) => {
                match compile_user_bindings(&contents, self.platform) {
                    Ok(rules) => rules,
                    Err(error) => {
                        let diagnostic = format!("rejected {}: {error}", self.path.display());
                        self.diagnostics = vec![diagnostic.clone()];
                        return KeybindingsResourcePoll::Rejected(diagnostic);
                    }
                }
            }
        };
        self.diagnostics = binding_diagnostics(&rules, self.platform);
        keybindings.replace_user_bindings(rules);
        KeybindingsResourcePoll::Updated
    }

    pub(crate) const fn next_deadline(&self) -> Instant {
        self.next_poll
    }

    pub(crate) fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub(crate) fn update_command_binding(
        &mut self,
        command: NativeCommand,
        keybinding: &zeta_keybinding::KeySequence,
        now: Instant,
    ) -> Result<(), String> {
        let mut value = match read_snapshot(&self.path).map_err(|error| error.to_string())? {
            ResourceSnapshot::Missing => Value::Array(Vec::new()),
            ResourceSnapshot::Contents(contents) => serde_json::from_slice(&contents)
                .map_err(|error| format!("cannot edit invalid keybindings JSON: {error}"))?,
        };
        let bindings = value
            .as_array_mut()
            .ok_or_else(|| "cannot edit keybindings because the root is not an array".to_owned())?;
        bindings.retain(|entry| {
            entry
                .as_object()
                .and_then(|entry| entry.get("command"))
                .and_then(Value::as_str)
                != Some(command.id())
        });
        bindings.push(Value::Object(Map::from_iter([
            (
                "key".to_owned(),
                Value::String(serialize_key_sequence(keybinding)),
            ),
            ("command".to_owned(), Value::String(command.id().to_owned())),
        ])));
        let contents = serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("could not serialize keybindings: {error}"))?;
        write_atomically(&self.path, &contents)
            .map_err(|error| format!("could not save {}: {error}", self.path.display()))?;
        self.observed = None;
        self.next_poll = now;
        Ok(())
    }
}

fn binding_diagnostics(rules: &[NativeUserBinding], platform: HostPlatform) -> Vec<String> {
    let mut diagnostics = Vec::new();
    for (later_index, later) in rules.iter().enumerate() {
        for (earlier_index, earlier) in rules[..later_index].iter().enumerate() {
            if earlier.keybinding == later.keybinding && earlier.when == later.when {
                let key = format_key_sequence(&later.keybinding, platform);
                let condition = later.when_source.as_deref().unwrap_or("always");
                diagnostics.push(format!(
                    "Rule {} conflicts with rule {} for {key} when {condition}; the later rule wins",
                    later_index + 1,
                    earlier_index + 1
                ));
            }
        }
    }
    diagnostics
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceError {
    InvalidJson(String),
    ExpectedArray,
    TooManyBindings { maximum: usize, actual: usize },
    ExpectedObject { index: usize },
    UnknownField { index: usize, field: String },
    MissingField { index: usize, field: &'static str },
    InvalidField { index: usize, field: &'static str },
    InvalidKey { index: usize, message: String },
    UnknownCommand { index: usize, command: String },
    UnknownCondition { index: usize, condition: String },
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(formatter, "invalid JSON: {message}"),
            Self::ExpectedArray => formatter.write_str("the root value must be an array"),
            Self::TooManyBindings { maximum, actual } => {
                write!(
                    formatter,
                    "at most {maximum} bindings are allowed, got {actual}"
                )
            }
            Self::ExpectedObject { index } => {
                write!(formatter, "binding {index} must be an object")
            }
            Self::UnknownField { index, field } => {
                write!(
                    formatter,
                    "binding {index} contains unknown field `{field}`"
                )
            }
            Self::MissingField { index, field } => {
                write!(formatter, "binding {index} is missing `{field}`")
            }
            Self::InvalidField { index, field } => {
                write!(formatter, "binding {index} has an invalid `{field}`")
            }
            Self::InvalidKey { index, message } => {
                write!(formatter, "binding {index} has an invalid key: {message}")
            }
            Self::UnknownCommand { index, command } => {
                write!(
                    formatter,
                    "binding {index} uses unknown command `{command}`"
                )
            }
            Self::UnknownCondition { index, condition } => {
                write!(
                    formatter,
                    "binding {index} uses unknown condition `{condition}`"
                )
            }
        }
    }
}

fn read_snapshot(path: &Path) -> io::Result<ResourceSnapshot> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ResourceSnapshot::Missing);
        }
        Err(error) => return Err(error),
    };
    if metadata.len() > MAX_RESOURCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("resource exceeds {MAX_RESOURCE_BYTES} bytes"),
        ));
    }
    let contents = fs::read(path)?;
    if contents.len() as u64 > MAX_RESOURCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("resource exceeds {MAX_RESOURCE_BYTES} bytes"),
        ));
    }
    Ok(ResourceSnapshot::Contents(contents))
}

fn compile_user_bindings(
    contents: &[u8],
    platform: HostPlatform,
) -> Result<Vec<NativeUserBinding>, ResourceError> {
    let value: Value = serde_json::from_slice(contents)
        .map_err(|error| ResourceError::InvalidJson(error.to_string()))?;
    let values = value.as_array().ok_or(ResourceError::ExpectedArray)?;
    if values.len() > MAX_BINDINGS {
        return Err(ResourceError::TooManyBindings {
            maximum: MAX_BINDINGS,
            actual: values.len(),
        });
    }
    values
        .iter()
        .enumerate()
        .filter_map(
            |(index, value)| match compile_user_binding(value, index + 1, platform) {
                Ok(Some(rule)) => Some(Ok(rule)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect()
}

fn compile_user_binding(
    value: &Value,
    index: usize,
    platform: HostPlatform,
) -> Result<Option<NativeUserBinding>, ResourceError> {
    let object = value
        .as_object()
        .ok_or(ResourceError::ExpectedObject { index })?;
    reject_unknown_fields(object, index)?;
    validate_platform_overrides(object, index)?;
    let key = selected_key(object, index, platform)?;
    let Some(key) = key else {
        return Ok(None);
    };
    let keybinding = parse_key_sequence(key).map_err(|error| ResourceError::InvalidKey {
        index,
        message: error.to_string(),
    })?;
    let command = object.get("command").ok_or(ResourceError::MissingField {
        index,
        field: "command",
    })?;
    let target = if command.is_null() {
        NativeUserBindingTarget::Block
    } else {
        let command = command.as_str().ok_or(ResourceError::InvalidField {
            index,
            field: "command",
        })?;
        NativeCommand::bindable_from_id(command)
            .map(NativeUserBindingTarget::Command)
            .ok_or_else(|| ResourceError::UnknownCommand {
                index,
                command: command.to_owned(),
            })?
    };
    let when_source = object
        .get("when")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let when = match when_source.as_deref() {
        None => NativeBindingCondition::Always,
        Some(condition) => {
            let expression = ContextExpression::parse(condition).map_err(|error| {
                ResourceError::UnknownCondition {
                    index,
                    condition: error.to_string(),
                }
            })?;
            if let Some(key) = expression
                .referenced_keys()
                .into_iter()
                .find(|key| !NativeKeybindingContext::supports_key(key))
            {
                return Err(ResourceError::UnknownCondition {
                    index,
                    condition: format!("unknown context key `{key}`"),
                });
            }
            NativeBindingCondition::Expression(expression)
        }
    };
    if object.get("when").is_some_and(|value| !value.is_string()) {
        return Err(ResourceError::InvalidField {
            index,
            field: "when",
        });
    }
    Ok(Some(NativeUserBinding {
        keybinding,
        target,
        when,
        when_source,
    }))
}

fn selected_key(
    object: &Map<String, Value>,
    index: usize,
    platform: HostPlatform,
) -> Result<Option<&str>, ResourceError> {
    let key = object
        .get("key")
        .ok_or(ResourceError::MissingField {
            index,
            field: "key",
        })?
        .as_str()
        .ok_or(ResourceError::InvalidField {
            index,
            field: "key",
        })?;
    let field = match platform {
        HostPlatform::MacOs => "mac",
        HostPlatform::Windows => "win",
        HostPlatform::Linux => "linux",
        HostPlatform::Other => return Ok(Some(key)),
    };
    let Some(value) = object.get(field) else {
        return Ok(Some(key));
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(Some)
        .ok_or(ResourceError::InvalidField { index, field })
}

fn reject_unknown_fields(object: &Map<String, Value>, index: usize) -> Result<(), ResourceError> {
    for field in object.keys() {
        if !matches!(
            field.as_str(),
            "key" | "command" | "when" | "mac" | "linux" | "win"
        ) {
            return Err(ResourceError::UnknownField {
                index,
                field: field.clone(),
            });
        }
    }
    Ok(())
}

fn validate_platform_overrides(
    object: &Map<String, Value>,
    index: usize,
) -> Result<(), ResourceError> {
    for field in ["mac", "linux", "win"] {
        if let Some(value) = object.get(field)
            && !value.is_null()
            && !value.is_string()
        {
            return Err(ResourceError::InvalidField { index, field });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "keybindings_resource_tests.rs"]
mod tests;
