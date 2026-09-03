mod model;
mod panel;
mod panel_request;
mod settings;
mod settings_request;
mod setup;
mod view;

pub(crate) use model::StatusLineModel;
pub(crate) use model::StatusLineRuntime;
pub(crate) use panel::RemainingContextWindow;
pub(crate) use panel::StatusPanel;
pub(crate) use panel::StatusViewData;
pub(crate) use panel::status_panel;
pub(crate) use panel_request::StatusRequestScope;
pub(crate) use panel_request::load_status_panel;
pub(crate) use settings::StatusLineItem;
pub(crate) use settings::StatusLineSettings;
pub(crate) use settings_request::StatusLineEdit;
pub(crate) use settings_request::StatusLineEditorUpdate;
pub(crate) use settings_request::read_status_line;
pub(crate) use settings_request::set_status_line;
pub(crate) use setup::StatusLineChoices;
pub(crate) use setup::StatusLineSelectionAction;
#[cfg(test)]
pub(crate) use setup::list_selection as status_line_choices;
pub(crate) use view::draw;
