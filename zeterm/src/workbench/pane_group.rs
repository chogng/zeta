//! Compatibility boundary for zeterm's workbench host.
//!
//! Topology and split-layout state lives in [`zeta_workbench`]. The product keeps this thin module
//! so existing host and presentation code can migrate without reintroducing a second owner.

pub(crate) use zeta_workbench::PaneGroup;
pub(crate) use zeta_workbench::PaneId;
pub(crate) use zeta_workbench::PaneSplitDirection;
pub(crate) use zeta_workbench::PaneSplitId;
