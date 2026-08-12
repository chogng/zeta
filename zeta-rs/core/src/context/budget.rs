use std::fmt;

/// A token quantity used by the context planner.
///
/// This newtype keeps context-window, output, and safety allocations explicit at call sites.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ContextTokenCount(u32);

impl ContextTokenCount {
    pub const ZERO: Self = Self(0);

    pub const fn new(tokens: u32) -> Self {
        Self(tokens)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for ContextTokenCount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Selects the input pressure boundary independently from the model's hard context window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextCompactionLimit {
    ContextWindow,
    Tokens(ContextTokenCount),
}

/// Immutable token allocations for one context-planning operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextBudget {
    /// The provider owns overflow handling because the selected model has no verified window.
    ProviderManaged,
    /// Core owns deterministic selection and compaction at the supplied limits.
    CoreManaged {
        context_window: ContextTokenCount,
        reserved_output: ContextTokenCount,
        safety_margin: ContextTokenCount,
        compaction_limit: ContextCompactionLimit,
    },
}

impl ContextBudget {
    pub const fn core_managed(
        context_window: ContextTokenCount,
        reserved_output: ContextTokenCount,
        safety_margin: ContextTokenCount,
        compaction_limit: ContextCompactionLimit,
    ) -> Self {
        Self::CoreManaged {
            context_window,
            reserved_output,
            safety_margin,
            compaction_limit,
        }
    }

    pub const fn provider_managed() -> Self {
        Self::ProviderManaged
    }

    pub(crate) fn limits(self) -> Option<CoreManagedContextBudget> {
        let Self::CoreManaged {
            context_window,
            reserved_output,
            safety_margin,
            compaction_limit,
        } = self
        else {
            return None;
        };
        Some(CoreManagedContextBudget {
            context_window,
            reserved_output,
            safety_margin,
            compaction_limit,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CoreManagedContextBudget {
    context_window: ContextTokenCount,
    reserved_output: ContextTokenCount,
    safety_margin: ContextTokenCount,
    compaction_limit: ContextCompactionLimit,
}

impl CoreManagedContextBudget {
    pub(crate) const fn context_window(self) -> ContextTokenCount {
        self.context_window
    }

    pub(crate) const fn reserved_output(self) -> ContextTokenCount {
        self.reserved_output
    }

    pub(crate) const fn safety_margin(self) -> ContextTokenCount {
        self.safety_margin
    }

    pub(crate) fn maximum_input(self) -> Option<ContextTokenCount> {
        let hard_input = self.maximum_compaction_input()?;
        let pressure_input = match self.compaction_limit {
            ContextCompactionLimit::ContextWindow => hard_input,
            ContextCompactionLimit::Tokens(limit) => {
                let pressure_limit = limit.get().min(self.context_window.get());
                let after_output =
                    pressure_limit.checked_sub(self.reserved_output.get().min(pressure_limit))?;
                ContextTokenCount::new(after_output.saturating_sub(self.safety_margin.get()))
            }
        };
        Some(pressure_input)
    }

    /// Maximum input for the compaction invocation itself, independent of the pressure threshold.
    pub(crate) fn maximum_compaction_input(self) -> Option<ContextTokenCount> {
        self.context_window
            .get()
            .checked_sub(self.reserved_output.get())?
            .checked_sub(self.safety_margin.get())
            .map(ContextTokenCount::new)
    }
}
