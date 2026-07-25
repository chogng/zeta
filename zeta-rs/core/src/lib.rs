//! Zeta's domain state machines, Thread manager, and outbound ports.

mod capabilities;
mod error;
mod ports;
mod state;
mod thread_manager;

pub use error::CoreError;
pub use ports::AgentModel;
pub use ports::ApprovalPolicy;
pub use ports::ApprovalRequirement;
pub use ports::EventJournal;
pub use ports::IdempotencyLedger;
pub use ports::IdempotencyRecord;
pub use ports::LeaseGuard;
pub use ports::ThreadWriterLease;
pub use state::ItemStatus;
pub use state::ToolCallStatus;
pub use state::TurnStatus;
pub use thread_manager::InMemoryIdempotencyLedger;
pub use thread_manager::InMemoryJournal;
pub use thread_manager::ThreadManager;
pub use thread_manager::ThreadSnapshot;

#[cfg(test)]
#[path = "thread_manager_tests.rs"]
mod tests;
pub use capabilities::BrowserAction;
pub use capabilities::BrowserActionResult;
pub use capabilities::BrowserCapability;
pub use capabilities::BrowserError;
pub use capabilities::BrowserObservation;
pub use capabilities::BrowserObserveRequest;
pub use capabilities::BrowserTargetId;
pub use capabilities::ElementTarget;
pub use capabilities::GetPdfRequest;
pub use capabilities::PdfResource;
pub use capabilities::TextInputTarget;
pub use capabilities::UnsupportedBrowserCapability;
