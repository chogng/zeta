use std::fmt;
use std::future::{Future, ready};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BrowserTargetId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementTarget {
    pub node_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserObserveRequest {
    pub target_id: BrowserTargetId,
    pub include_accessibility_tree: bool,
    pub include_dom_snapshot: bool,
    pub include_screenshot: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserObservation {
    pub target_id: BrowserTargetId,
    pub accessibility_tree: Option<String>,
    pub dom_snapshot: Option<String>,
    pub screenshot: Option<PdfResource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextInputTarget {
    Element(ElementTarget),
    FocusedElement,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserAction {
    Navigate {
        target_id: BrowserTargetId,
        url: String,
    },
    Click {
        target_id: BrowserTargetId,
        target: ElementTarget,
    },
    TypeText {
        target_id: BrowserTargetId,
        target: TextInputTarget,
        text: String,
    },
    Scroll {
        target_id: BrowserTargetId,
        delta_x: f64,
        delta_y: f64,
    },
    GoBack {
        target_id: BrowserTargetId,
    },
    Reload {
        target_id: BrowserTargetId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserActionResult {
    pub target_id: BrowserTargetId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetPdfRequest {
    pub target_id: BrowserTargetId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfResource {
    pub resource_id: String,
    pub mime_type: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserError {
    CapabilityUnavailable,
    TargetUnavailable(BrowserTargetId),
    PolicyDenied(String),
    Failed(String),
}

impl fmt::Display for BrowserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapabilityUnavailable => formatter.write_str("browser capability unavailable"),
            Self::TargetUnavailable(target) => {
                write!(formatter, "browser target unavailable: {}", target.0)
            }
            Self::PolicyDenied(reason) => {
                write!(formatter, "browser action denied by policy: {reason}")
            }
            Self::Failed(reason) => formatter.write_str(reason),
        }
    }
}
impl std::error::Error for BrowserError {}

/// Host-provided browser operations used by the Agent.
///
/// Implementations translate these semantic requests into a local or remote browser backend while
/// enforcing target ownership, origin policy, deadlines, and cancellation. They must never expose
/// arbitrary CDP commands to Core or let a closed target silently resolve to a different tab.
pub trait BrowserCapability: Send + Sync {
    fn observe(
        &self,
        request: BrowserObserveRequest,
    ) -> impl Future<Output = Result<BrowserObservation, BrowserError>> + Send;
    fn perform(
        &self,
        action: BrowserAction,
    ) -> impl Future<Output = Result<BrowserActionResult, BrowserError>> + Send;
    fn get_pdf(
        &self,
        request: GetPdfRequest,
    ) -> impl Future<Output = Result<PdfResource, BrowserError>> + Send;
}

pub struct UnsupportedBrowserCapability;
impl BrowserCapability for UnsupportedBrowserCapability {
    fn observe(
        &self,
        _: BrowserObserveRequest,
    ) -> impl Future<Output = Result<BrowserObservation, BrowserError>> + Send {
        ready(Err(BrowserError::CapabilityUnavailable))
    }
    fn perform(
        &self,
        _: BrowserAction,
    ) -> impl Future<Output = Result<BrowserActionResult, BrowserError>> + Send {
        ready(Err(BrowserError::CapabilityUnavailable))
    }
    fn get_pdf(
        &self,
        _: GetPdfRequest,
    ) -> impl Future<Output = Result<PdfResource, BrowserError>> + Send {
        ready(Err(BrowserError::CapabilityUnavailable))
    }
}
