use std::fs;
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use serde_json::Map;
use serde_json::Value;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;
use zeta_keybinding::serialize_key_sequence;
use zeta_utils_path::write_atomically;

use crate::catalog::KeybindingCatalog;
use crate::engine::Keybindings;
use crate::engine::UserBinding;
use crate::engine::UserBindingTarget;

const MAX_RESOURCE_BYTES: u64 = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSnapshot {
    Missing,
    Contents(Vec<u8>),
}

/// The result of checking a user keybinding resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeybindingsResourcePoll {
    Unchanged,
    Updated,
    Rejected(String),
}

/// A bounded, polling JSON resource for one product keybinding catalog.
pub struct KeybindingsResource<C: KeybindingCatalog> {
    path: PathBuf,
    platform: HostPlatform,
    observed: Option<ResourceSnapshot>,
    diagnostics: Vec<String>,
    next_poll: Instant,
    catalog: PhantomData<C>,
}

impl<C: KeybindingCatalog> KeybindingsResource<C> {
    pub fn new(path: PathBuf, platform: HostPlatform, now: Instant) -> Self {
        Self {
            path,
            platform,
            observed: None,
            diagnostics: Vec::new(),
            next_poll: now,
            catalog: PhantomData,
        }
    }

    pub fn poll(
        &mut self,
        now: Instant,
        keybindings: &mut Keybindings<C>,
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
                match compile_user_bindings::<C>(&contents, self.platform) {
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

    pub const fn next_deadline(&self) -> Instant {
        self.next_poll
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn update_command_binding(
        &mut self,
        command: C::Command,
        keybinding: &KeySequence,
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
                != Some(C::command_id(command))
        });
        bindings.push(Value::Object(Map::from_iter([
            (
                "key".to_owned(),
                Value::String(serialize_key_sequence(keybinding)),
            ),
            (
                "command".to_owned(),
                Value::String(C::command_id(command).to_owned()),
            ),
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

/// Shared validation error for a malformed user keybinding resource.
pub type KeybindingsResourceError = zeta_keybinding::UserBindingsError;

/// Parses a complete user resource into product rules without mutating an active engine.
pub fn compile_user_bindings<C: KeybindingCatalog>(
    contents: &[u8],
    platform: HostPlatform,
) -> Result<Vec<UserBinding<C>>, KeybindingsResourceError> {
    zeta_keybinding::compile_user_bindings(
        contents,
        platform,
        C::command_from_id,
        C::parse_condition,
    )
    .map(|rules| {
        rules
            .into_iter()
            .map(|rule| UserBinding {
                keybinding: rule.keybinding,
                target: match rule.target {
                    zeta_keybinding::UserBindingTarget::Command(command) => {
                        UserBindingTarget::Command(command)
                    }
                    zeta_keybinding::UserBindingTarget::Block => UserBindingTarget::Block,
                },
                when: rule.when,
                when_source: rule.when_source,
            })
            .collect()
    })
}

/// Produces non-fatal diagnostics for exact duplicate rules.
pub fn binding_diagnostics<C: KeybindingCatalog>(
    rules: &[UserBinding<C>],
    platform: HostPlatform,
) -> Vec<String> {
    let shared_rules = rules
        .iter()
        .map(|rule| zeta_keybinding::UserBinding {
            keybinding: rule.keybinding.clone(),
            target: match rule.target {
                UserBindingTarget::Command(command) => {
                    zeta_keybinding::UserBindingTarget::Command(command)
                }
                UserBindingTarget::Block => zeta_keybinding::UserBindingTarget::Block,
            },
            when: rule.when.clone(),
            when_source: rule.when_source.clone(),
        })
        .collect::<Vec<_>>();
    zeta_keybinding::user_binding_diagnostics(&shared_rules, platform)
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
