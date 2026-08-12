use crate::ContentDigest;
use crate::SkillCatalog;
use crate::SkillError;
use crate::SkillErrorKind;
use std::fs;
use std::fs::File;
use std::io::Read;
use zeta_file_identity::FileInformation;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillVersionSelector;

const MAX_SKILL_FILE_BYTES: u64 = 1024 * 1024;

/// Exact validated Skill instructions bound to durable activation provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivatedSkill {
    activation: FrozenSkillActivation,
    body: String,
}

impl ActivatedSkill {
    pub fn activation(&self) -> &FrozenSkillActivation {
        &self.activation
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

impl SkillCatalog {
    /// Loads one catalog entry through its controlled source root and freezes its exact digest.
    pub fn activate(
        &self,
        selected: &SkillRef,
        reason: SkillActivationReason,
    ) -> Result<ActivatedSkill, SkillError> {
        let entry = self.snapshot.read(&selected.id).ok_or_else(|| {
            SkillError::new(
                SkillErrorKind::SkillNotFound,
                format!(
                    "Skill '{}' is not present in the current catalog",
                    selected.id.name
                ),
            )
        })?;
        if let SkillVersionSelector::PinnedDigest { digest } = &selected.version
            && digest != entry.content_digest()
        {
            return Err(content_changed(&selected.id.name.to_string()));
        }
        let source = self
            .sources
            .iter()
            .find(|source| source.view().id() == &selected.id.source)
            .ok_or_else(|| {
                SkillError::new(
                    SkillErrorKind::SourceUnavailable,
                    format!("Skill source '{}' is unavailable", selected.id.source),
                )
            })?;
        let body = load_exact_body(
            source.canonical_root(),
            selected.id.name.as_str(),
            entry.content_digest(),
        )?;
        Ok(ActivatedSkill {
            activation: FrozenSkillActivation {
                id: selected.id.clone(),
                content_digest: entry.content_digest().clone(),
                catalog_generation: self.snapshot.generation().get(),
                reason,
            },
            body,
        })
    }
}

fn load_exact_body(
    source_root: &std::path::Path,
    skill_name: &str,
    expected_digest: &ContentDigest,
) -> Result<String, SkillError> {
    let directory = source_root.join(skill_name);
    let directory_metadata = fs::symlink_metadata(&directory)
        .map_err(|_| unavailable(skill_name, "directory is unavailable"))?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err(unavailable(skill_name, "directory is not a real directory"));
    }
    let canonical_directory = directory
        .canonicalize()
        .map_err(|_| unavailable(skill_name, "directory cannot be resolved"))?;
    if !canonical_directory.starts_with(source_root) {
        return Err(unavailable(skill_name, "directory escapes its source"));
    }

    let path = directory.join("SKILL.md");
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| unavailable(skill_name, "SKILL.md is unavailable"))?;
    let information = FileInformation::from_path(&path)
        .map_err(|_| unavailable(skill_name, "SKILL.md cannot be inspected"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || information.number_of_links() > 1
        || metadata.len() > MAX_SKILL_FILE_BYTES
    {
        return Err(unavailable(
            skill_name,
            "SKILL.md is not an admissible regular file",
        ));
    }
    let canonical_path = path
        .canonicalize()
        .map_err(|_| unavailable(skill_name, "SKILL.md cannot be resolved"))?;
    if !canonical_path.starts_with(&canonical_directory) {
        return Err(unavailable(
            skill_name,
            "SKILL.md escapes its Skill directory",
        ));
    }

    let mut file =
        File::open(&path).map_err(|_| unavailable(skill_name, "SKILL.md cannot be opened"))?;
    let opened = FileInformation::from_file(&file)
        .map_err(|_| unavailable(skill_name, "SKILL.md cannot be inspected"))?;
    if opened.identity() != information.identity() || opened.number_of_links() > 1 {
        return Err(content_changed(skill_name));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take(MAX_SKILL_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable(skill_name, "SKILL.md cannot be read"))?;
    let observed = fs::symlink_metadata(&path).map_err(|_| content_changed(skill_name))?;
    let observed_information =
        FileInformation::from_path(&path).map_err(|_| content_changed(skill_name))?;
    if bytes.len() as u64 > MAX_SKILL_FILE_BYTES
        || observed.file_type().is_symlink()
        || !observed.is_file()
        || observed.len() != bytes.len() as u64
        || observed_information.identity() != opened.identity()
        || observed_information.number_of_links() > 1
    {
        return Err(content_changed(skill_name));
    }
    if &ContentDigest::sha256(&bytes) != expected_digest {
        return Err(content_changed(skill_name));
    }
    String::from_utf8(bytes).map_err(|_| {
        SkillError::new(
            SkillErrorKind::InvalidContent,
            format!("Skill '{skill_name}' SKILL.md is not valid UTF-8"),
        )
    })
}

fn unavailable(skill_name: &str, detail: &str) -> SkillError {
    SkillError::new(
        SkillErrorKind::SourceUnavailable,
        format!("Skill '{skill_name}' {detail}"),
    )
}

fn content_changed(skill_name: &str) -> SkillError {
    SkillError::new(
        SkillErrorKind::ContentChanged,
        format!("Skill '{skill_name}' content changed after selection"),
    )
}

#[cfg(test)]
#[path = "activation_tests.rs"]
mod tests;
