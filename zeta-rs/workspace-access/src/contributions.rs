/// Optional project contribution that an additional directory may expose.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdditionalDirectoryContribution {
    Skills,
    AgentDefinitions,
    EnabledPlugins,
    ExtraKnownMarketplaces,
    ProjectInstructions,
    InstructionRules,
    LocalInstructions,
}

/// Host compatibility policy for instruction files in additional directories.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AdditionalInstructionsPolicy {
    Exclude,
    Include,
}

/// Configuration surface exposed by one effective additional-directory entry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AdditionalDirectoryContributionPolicy {
    FileAccessOnly,
    AllowlistedProjectContributions,
    AllowlistedProjectContributionsWithInstructions,
}

impl AdditionalDirectoryContributionPolicy {
    /// Reports whether this policy exposes one contribution kind.
    pub fn allows(self, contribution: AdditionalDirectoryContribution) -> bool {
        self.contributions().contains(&contribution)
    }

    /// Returns the complete stable allowlist for this policy.
    pub fn contributions(self) -> &'static [AdditionalDirectoryContribution] {
        use AdditionalDirectoryContribution::AgentDefinitions;
        use AdditionalDirectoryContribution::EnabledPlugins;
        use AdditionalDirectoryContribution::ExtraKnownMarketplaces;
        use AdditionalDirectoryContribution::InstructionRules;
        use AdditionalDirectoryContribution::LocalInstructions;
        use AdditionalDirectoryContribution::ProjectInstructions;
        use AdditionalDirectoryContribution::Skills;

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
