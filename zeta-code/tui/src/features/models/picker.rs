use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::search_box::SearchBoxModel;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::model::ModelListResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ModelSelectionAction {
    Select { preference: String },
}

pub(crate) struct ModelChoices {
    pub(crate) model: ListSelectionModel,
    pub(crate) actions: BTreeMap<ListSelectionItemId, ModelSelectionAction>,
}

pub(crate) fn model_choices(
    catalog: &ModelListResult,
    preferred_model: Option<&ModelRefDto>,
) -> ModelChoices {
    let current = preferred_model.map(|model| format!("{}/{}", model.provider, model.model));
    let mut actions = BTreeMap::new();
    let automatic_id = ListSelectionItemId::new("model-automatic");
    actions.insert(
        automatic_id.clone(),
        ModelSelectionAction::Select {
            preference: "clear".into(),
        },
    );
    let automatic_selected = current.is_none();
    let mut selected = 0;
    let mut items = vec![
        ListSelectionItem::new(format!(
            "Automatic{}",
            if automatic_selected { " ✓" } else { "" }
        ))
        .with_id(automatic_id)
        .with_description("use the product default"),
    ];
    items.extend(catalog.models.iter().enumerate().map(|(index, entry)| {
        let preference = format!("{}/{}", entry.model.provider, entry.model.model);
        let is_selected = current.as_deref() == Some(preference.as_str());
        if is_selected {
            selected = index + 1;
        }
        let item_id = ListSelectionItemId::new(format!("model-{index}"));
        actions.insert(
            item_id.clone(),
            ModelSelectionAction::Select {
                preference: preference.clone(),
            },
        );
        ListSelectionItem::new(format!(
            "{}{}",
            entry.display_name,
            if is_selected { " ✓" } else { "" }
        ))
        .with_id(item_id)
        .with_description(preference)
    }));

    ModelChoices {
        model: ListSelectionModel::new("Model", vec![ListSelectionGroup::new("Models", items)])
            .with_activation_mode(ListSelectionActivationMode::Enter)
            .with_activation_label("apply")
            .without_tab_bar()
            .with_initial_selected(selected)
            .with_search(SearchBoxModel::new("Search models"))
            .with_empty_message("No matching models"),
        actions,
    }
}

#[cfg(test)]
#[path = "picker_tests.rs"]
mod tests;
