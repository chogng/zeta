/// Optional project contribution that an additional directory may expose.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdditionalDirectoryContribution {
    /// Skill directories that may contribute model-facing instructions.
    Skills,
    /// Named Agent definitions that may be selected for delegated execution.
    AgentDefinitions,
    /// The `enabledPlugins` declaration from an allowlisted settings projection.
    EnabledPlugins,
    /// The `extraKnownMarketplaces` declaration from an allowlisted settings projection.
    ExtraKnownMarketplaces,
    /// Project instruction files such as `CLAUDE.md`.
    ProjectInstructions,
    /// Project instruction-rule directories.
    InstructionRules,
    /// Local project instruction overrides such as `CLAUDE.local.md`.
    LocalInstructions,
}

/// Host compatibility policy for instruction files in transient additional directories.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AdditionalInstructionsPolicy {
    /// Do not expose project, rule, or local instruction files.
    Exclude,
    /// Expose instruction files in addition to the base transient allowlist.
    Include,
}

/// Configuration surface exposed by one effective additional-directory entry.
///
/// Filesystem access is granted independently by the host. This policy only describes which
/// project contributions may be discovered after access has already been authorized.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AdditionalDirectoryContributionPolicy {
    /// Directory access does not permit any configuration discovery.
    FileAccessOnly,
    /// Transient source exposing Skills, Agent definitions, and selected Plugin declarations.
    AllowlistedProjectContributions,
    /// Base transient allowlist plus instruction files enabled by host compatibility policy.
    AllowlistedProjectContributionsWithInstructions,
}

impl AdditionalDirectoryContributionPolicy {
    /// Reports whether this policy exposes one contribution kind.
    pub fn allows(self, contribution: AdditionalDirectoryContribution) -> bool {
        self.contributions().contains(&contribution)
    }

    /// Returns the complete, stable allowlist for this policy.
    pub fn contributions(self) -> &'static [AdditionalDirectoryContribution] {
        use AdditionalDirectoryContribution::{
            AgentDefinitions, EnabledPlugins, ExtraKnownMarketplaces, InstructionRules,
            LocalInstructions, ProjectInstructions, Skills,
        };

        match self {
            Self::FileAccessOnly => &[],
            Self::AllowlistedProjectContributions => &[
                Skills,
                AgentDefinitions,
                EnabledPlugins,
                ExtraKnownMarketplaces,
            ],
            Self::AllowlistedProjectContributionsWithInstructions => &[
                Skills,
                AgentDefinitions,
                EnabledPlugins,
                ExtraKnownMarketplaces,
                ProjectInstructions,
                InstructionRules,
                LocalInstructions,
            ],
        }
    }
}
