//! Logical TabPart state.

mod tab_group;
mod tab_input;
mod tab_part;

pub use tab_group::{TabGroup, TabGroupId};
pub use tab_input::{TabInput, TabInputChange, TabInputKey, TabInputMetadata};
pub use tab_part::TabPart;
