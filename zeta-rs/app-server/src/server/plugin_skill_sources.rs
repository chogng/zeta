use zeta_plugins::PluginActivationAuthority;
use zeta_protocol::SkillName;
use zeta_protocol::SkillSourceId;
use zeta_skills::SkillSourceRoot;
use zeta_skills_extension::DynamicSkillSourceProvider;
use zeta_skills_extension::DynamicSkillSourceSnapshot;

/// Projects exact effective Plugin Skill contributions into the shared Skill runtime.
pub(super) struct PluginSkillSourceProvider {
    authority: PluginActivationAuthority,
}

impl PluginSkillSourceProvider {
    pub(super) fn new(authority: PluginActivationAuthority) -> Self {
        Self { authority }
    }
}

impl DynamicSkillSourceProvider for PluginSkillSourceProvider {
    fn snapshot(&self) -> Result<DynamicSkillSourceSnapshot, String> {
        let activation = self.authority.snapshot().activation().clone();
        let mut roots = Vec::new();
        for package in activation.packages() {
            for contribution in &package.manifest().contributions.skills {
                let source = SkillSourceId::new(format!(
                    "plugin-{}:skill-source:{}",
                    package.manifest().id,
                    contribution.id
                ))
                .map_err(|error| error.to_string())?;
                let name =
                    SkillName::new(contribution.id.as_str()).map_err(|error| error.to_string())?;
                let root = package
                    .resolve_directory(&contribution.path)
                    .map_err(|error| error.to_string())?;
                roots.push(
                    SkillSourceRoot::plugin(source, name, root)
                        .map_err(|error| error.to_string())?,
                );
            }
        }
        Ok(DynamicSkillSourceSnapshot {
            generation: activation.generation(),
            roots,
        })
    }
}

#[cfg(test)]
#[path = "plugin_skill_sources_tests.rs"]
mod tests;
