use std::collections::BTreeSet;

/// Optional project contribution that a directory may expose.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Contribution {
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

/// Configuration surface exposed by one effective directory entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Contributions {
    entries: BTreeSet<Contribution>,
}

impl Contributions {
    pub(crate) fn new(contributions: impl IntoIterator<Item = Contribution>) -> Self {
        Self {
            entries: contributions.into_iter().collect(),
        }
    }

    pub fn allows(&self, contribution: Contribution) -> bool {
        self.entries.contains(&contribution)
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = Contribution> + '_ {
        self.entries.iter().copied()
    }
}
