mod picker;
mod request;

/// A completed model operation delivered to the TUI state owner.
pub(crate) enum Event {
    SummaryReceived(ModelSummary),
    PickerOpened(ModelChoices),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    SetPreferred { preference: String },
}

use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_protocol::ModelAccess;

pub(crate) use picker::ModelChoices;
pub(crate) use picker::ModelSelectionAction;
pub(crate) use picker::model_choices;
pub(crate) use request::PreferredModelUpdate;
pub(crate) use request::load_selection;
pub(crate) use request::preferred_model_label;
pub(crate) use request::set_preferred_model;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelSummary {
    preferred_model: Option<ModelRefDto>,
    access: ModelAccess,
}

impl ModelSummary {
    pub(crate) fn from_catalog(
        preferred_model: Option<ModelRefDto>,
        catalog: Option<&ModelListResult>,
    ) -> Self {
        let access = preferred_model
            .as_ref()
            .and_then(|preferred| {
                catalog.and_then(|catalog| {
                    catalog.models.iter().find(|entry| {
                        entry.model.provider.as_str() == preferred.provider
                            && entry.model.model.as_str() == preferred.model
                    })
                })
            })
            .map(|entry| entry.access)
            .unwrap_or(ModelAccess::Unknown);
        Self {
            preferred_model,
            access,
        }
    }

    pub(crate) fn preferred_model(&self) -> Option<&ModelRefDto> {
        self.preferred_model.as_ref()
    }

    pub(crate) fn model_label(&self) -> String {
        self.preferred_model
            .as_ref()
            .map(|model| format!("{}/{}", model.provider, model.model))
            .unwrap_or_else(|| "Automatic model".into())
    }

    pub(crate) const fn access(&self) -> ModelAccess {
        self.access
    }
}

pub(crate) const fn access_label(access: ModelAccess) -> &'static str {
    match access {
        ModelAccess::ApiKey => "API usage billing",
        ModelAccess::Subscription => "Subscription",
        ModelAccess::Local => "Local",
        ModelAccess::Enterprise => "Enterprise",
        ModelAccess::Unknown => "Access unknown",
    }
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
