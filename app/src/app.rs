//! Native app application composition and lifecycle.
//!
//! The app module is the only product layer that coordinates zui lifecycle callbacks, feature
//! hosts, Workbench state, and the final presentation frame. Domain modules remain responsible
//! for their own state and adapters.

#[path = "app/native_app.rs"]
pub(crate) mod native_app;

pub use native_app::run;
