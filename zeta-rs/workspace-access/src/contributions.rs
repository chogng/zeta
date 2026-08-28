use std::collections::BTreeSet;

/// Optional project contribution that an additional directory may expose.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdditionalDirectoryContribution {
    Skills,
    AgentDefinitions,
    McpServers,
    LanguageServices,
    Hooks,
    EnabledPlugins,
    ExtraKnownMarketplaces,
    ProjectInstructions,
    InstructionRules,
    LocalInstructions,
}

/// Configuration surface exposed by one effective additional-directory entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AdditionalDirectoryContributionPolicy {
    contributions: BTreeSet<AdditionalDirectoryContribution>,
}

impl AdditionalDirectoryContributionPolicy {
    pub(crate) fn new(
        contributions: impl IntoIterator<Item = AdditionalDirectoryContribution>,
    ) -> Self {
        Self {
            contributions: contributions.into_iter().collect(),
        }
    }

    /// Reports whether this policy exposes one contribution kind.
    pub fn allows(&self, contribution: AdditionalDirectoryContribution) -> bool {
        self.contributions.contains(&contribution)
    }

    /// Returns the complete stable allowlist for this policy.
    pub fn contributions(
        &self,
    ) -> impl ExactSizeIterator<Item = AdditionalDirectoryContribution> + '_ {
        self.contributions.iter().copied()
    }
}
