use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_session::ComposerModelOption;

/// Normalizes transport-owned model catalog entries into the Session Pane contract.
pub(crate) fn session_model_options(entries: Vec<ModelCatalogEntry>) -> Vec<ComposerModelOption> {
    entries
        .into_iter()
        .map(|entry| ComposerModelOption {
            description: format!("{}/{}", entry.model.provider, entry.model.model),
            label: entry.display_name,
            model: entry.model,
        })
        .collect()
}
