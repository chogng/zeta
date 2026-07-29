use std::collections::BTreeSet;

use crate::{PluginError, PluginErrorKind};

use super::{ContributionKind, ContributionReference, ManifestLocalId, Permission, PluginManifest};

const SUPPORTED_SCHEMA_VERSION: u32 = 1;
const MAX_DISPLAY_NAME_BYTES: usize = 200;
const MAX_DESCRIPTION_BYTES: usize = 4096;
const MAX_LICENSE_BYTES: usize = 256;

impl PluginManifest {
    /// Revalidates semantic invariants after programmatic construction or modification.
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return invalid(format!(
                "unsupported plugin manifest schemaVersion {}; expected {}",
                self.schema_version, SUPPORTED_SCHEMA_VERSION
            ));
        }
        validate_plain_text(&self.display_name, "displayName", MAX_DISPLAY_NAME_BYTES)?;
        if let Some(description) = &self.description {
            validate_description(description)?;
        }
        if let Some(license) = &self.license {
            validate_plain_text(license, "license", MAX_LICENSE_BYTES)?;
        }
        if self.contributions.skills.is_empty()
            && self.contributions.mcp_servers.is_empty()
            && self.contributions.assets.is_empty()
        {
            return invalid("plugin manifest must declare at least one contribution");
        }

        unique_ids(
            "skill",
            self.contributions
                .skills
                .iter()
                .map(|contribution| &contribution.id),
        )?;
        unique_ids(
            "MCP server",
            self.contributions
                .mcp_servers
                .iter()
                .map(|contribution| &contribution.id),
        )?;
        unique_ids(
            "asset",
            self.contributions
                .assets
                .iter()
                .map(|contribution| &contribution.id),
        )?;
        unique_ids(
            "credential slot",
            self.credential_slots.iter().map(|slot| &slot.name),
        )?;

        let available = contribution_references(self);
        for slot in &self.credential_slots {
            let mut required = BTreeSet::new();
            for reference in &slot.required_for {
                if !required.insert(reference) {
                    return invalid(format!(
                        "credential slot '{}' repeats requiredFor reference '{}'",
                        slot.name, reference
                    ));
                }
                if !available.contains(reference) {
                    return invalid(format!(
                        "credential slot '{}' references missing contribution '{}'",
                        slot.name, reference
                    ));
                }
            }
        }

        let mut permissions = BTreeSet::new();
        for permission in &self.permissions {
            if !permissions.insert(permission) {
                return invalid("plugin manifest contains a duplicate permission");
            }
            if let Permission::Network { hosts } = permission {
                if hosts.is_empty() {
                    return invalid("network permission must declare at least one exact host");
                }
                let mut unique_hosts = BTreeSet::new();
                if hosts.iter().any(|host| !unique_hosts.insert(host)) {
                    return invalid("network permission contains a duplicate host");
                }
            }
        }

        for key in self.metadata.keys() {
            if !is_namespaced_metadata_key(key) {
                return invalid(format!(
                    "metadata key '{key}' must use a namespaced '<publisher>/<key>' form"
                ));
            }
        }
        Ok(())
    }
}

fn unique_ids<'a>(
    kind: &str,
    ids: impl IntoIterator<Item = &'a ManifestLocalId>,
) -> Result<(), PluginError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if !unique.insert(id) {
            return invalid(format!("duplicate {kind} contribution id '{id}'"));
        }
    }
    Ok(())
}

fn contribution_references(manifest: &PluginManifest) -> BTreeSet<ContributionReference> {
    manifest
        .contributions
        .skills
        .iter()
        .map(|item| ContributionReference {
            kind: ContributionKind::Skill,
            id: item.id.clone(),
        })
        .chain(
            manifest
                .contributions
                .mcp_servers
                .iter()
                .map(|item| ContributionReference {
                    kind: ContributionKind::Mcp,
                    id: item.id.clone(),
                }),
        )
        .chain(
            manifest
                .contributions
                .assets
                .iter()
                .map(|item| ContributionReference {
                    kind: ContributionKind::Asset,
                    id: item.id.clone(),
                }),
        )
        .collect()
}

fn validate_plain_text(value: &str, field: &str, max: usize) -> Result<(), PluginError> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return invalid(format!(
            "{field} must be non-empty plain text no longer than {max} bytes"
        ));
    }
    Ok(())
}

fn validate_description(value: &str) -> Result<(), PluginError> {
    if value.trim().is_empty()
        || value.len() > MAX_DESCRIPTION_BYTES
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return invalid(format!(
            "description must be non-empty text no longer than {MAX_DESCRIPTION_BYTES} bytes"
        ));
    }
    Ok(())
}

fn is_namespaced_metadata_key(key: &str) -> bool {
    let Some((namespace, name)) = key.split_once('/') else {
        return false;
    };
    key.matches('/').count() == 1 && is_metadata_segment(namespace) && is_metadata_segment(name)
}

fn is_metadata_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(['-', '.'])
        && !value.ends_with(['-', '.'])
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.' | b'_')
        })
}

fn invalid(message: impl Into<String>) -> Result<(), PluginError> {
    Err(PluginError::new(PluginErrorKind::ManifestInvalid, message))
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
