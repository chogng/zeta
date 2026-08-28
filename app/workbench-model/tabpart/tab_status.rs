/// Visual category used by Workbench to render one tab's current status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabStatusKind {
    Idle,
    Busy,
    Attention,
    Success,
    Warning,
    Error,
}

/// Product-supplied status shown by the Workbench tab chrome.
///
/// The product chooses the user-facing label and maps its runtime state into a visual category.
/// Workbench owns the category's color, motion, tooltip, and accessibility presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabStatus {
    kind: TabStatusKind,
    label: String,
}

impl Default for TabStatus {
    fn default() -> Self {
        Self::idle("Ready")
    }
}

impl TabStatus {
    pub fn new(kind: TabStatusKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
        }
    }

    pub fn idle(label: impl Into<String>) -> Self {
        Self::new(TabStatusKind::Idle, label)
    }

    pub fn busy(label: impl Into<String>) -> Self {
        Self::new(TabStatusKind::Busy, label)
    }

    pub fn attention(label: impl Into<String>) -> Self {
        Self::new(TabStatusKind::Attention, label)
    }

    pub fn success(label: impl Into<String>) -> Self {
        Self::new(TabStatusKind::Success, label)
    }

    pub fn warning(label: impl Into<String>) -> Self {
        Self::new(TabStatusKind::Warning, label)
    }

    pub fn error(label: impl Into<String>) -> Self {
        Self::new(TabStatusKind::Error, label)
    }

    pub const fn kind(&self) -> TabStatusKind {
        self.kind
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}
