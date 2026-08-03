/// The stable built-in prompt families owned by `zeta-prompts`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptCategory {
    /// Product and platform behavior supplied as the system-level baseline.
    System,
    /// Instructions used when producing a continuation summary.
    Compaction,
    /// Instructions governing an active task goal.
    Goals,
    /// General-purpose code and change review instructions.
    Review,
}

/// One immutable, compile-time embedded prompt asset.
///
/// The artifact carries identity and revision metadata so callers can include the exact prompt
/// revision in their invocation snapshot or other durable protocol. This type does not decide
/// when the asset is injected or which model message role receives it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PromptArtifact {
    category: PromptCategory,
    id: &'static str,
    revision: &'static str,
    body: &'static str,
}

impl PromptArtifact {
    pub(crate) const fn new(
        category: PromptCategory,
        id: &'static str,
        revision: &'static str,
        body: &'static str,
    ) -> Self {
        Self {
            category,
            id,
            revision,
            body,
        }
    }

    /// Returns the prompt family.
    pub const fn category(&self) -> PromptCategory {
        self.category
    }

    /// Returns the stable logical identifier for this asset.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Returns the revision that must change when prompt semantics change.
    pub const fn revision(&self) -> &'static str {
        self.revision
    }

    /// Returns the exact embedded prompt body, including its authored line ending.
    pub const fn body(&self) -> &'static str {
        self.body
    }

    pub(crate) fn render(self, body: String) -> RenderedPrompt {
        RenderedPrompt { source: self, body }
    }
}

/// A rendered prompt body that remains bound to its source asset and revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedPrompt {
    source: PromptArtifact,
    body: String,
}

impl RenderedPrompt {
    /// Returns the embedded asset from which this body was rendered.
    pub const fn source(&self) -> PromptArtifact {
        self.source
    }

    /// Returns the rendered prompt body.
    pub fn body(&self) -> &str {
        &self.body
    }
}
