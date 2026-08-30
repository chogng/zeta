mod state;
mod view;

#[cfg(test)]
#[path = "chat_composer_input_tests.rs"]
mod input_tests;

pub(crate) use state::ChatComposer;
pub(crate) use state::ChatComposerOutcome;
pub(crate) use state::ChatComposerOverlayView;
pub(crate) use state::ChatComposerPaneKind;
pub(crate) use state::ChatComposerPaneView;
pub(crate) use state::ChatComposerView;
pub(crate) use view::ChatComposerAreas;
pub(crate) use view::ChatComposerPointerTarget;
pub(crate) use view::draw;
pub(crate) use view::pointer_target_at;
pub(crate) use view::view_areas;
pub(crate) use view::view_desired_height;
