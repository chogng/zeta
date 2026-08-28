//! Top-level Tab groups, inputs, status, and selection state.

mod tab_group;
mod tab_input;
mod tab_part;
mod tab_status;

pub use tab_group::{TabGroup, TabGroupId};
pub use tab_input::{TabInput, TabInputChange, TabInputKey, TabInputMetadata};
pub use tab_part::{TabId, TabPart};
pub use tab_status::{TabStatus, TabStatusKind};
