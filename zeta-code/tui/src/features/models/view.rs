use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::model::ModelListResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ModelSelectionAction {
    Select { preference: String },
}

pub(crate) struct ModelSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ModelSelectionAction>,
}

pub(crate) fn model_selection_view(
    catalog: &ModelListResult,
    preferred_model: Option<&ModelRefDto>,
) -> ModelSelectionView {
    let current = preferred_model.map(|model| format!("{}/{}", model.provider, model.model));
    let mut actions = BTreeMap::new();
    let automatic_id = SelectionItemId::new("model-automatic");
    actions.insert(
        automatic_id.clone(),
        ModelSelectionAction::Select {
            preference: "clear".into(),
        },
    );
    let automatic_selected = current.is_none();
    let mut selected = 0;
    let mut items = vec![
        SelectionItem::new(format!(
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
        let item_id = SelectionItemId::new(format!("model-{index}"));
        actions.insert(
            item_id.clone(),
            ModelSelectionAction::Select {
                preference: preference.clone(),
            },
        );
        SelectionItem::new(format!(
            "{}{}",
            entry.display_name,
            if is_selected { " ✓" } else { "" }
        ))
        .with_id(item_id)
        .with_description(preference)
    }));

    ModelSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new("Model", vec![SelectionTab::new("Models", items)])
                .with_activation_mode(SelectionActivationMode::Enter)
                .without_tab_bar()
                .with_initial_selected(selected)
                .with_search(SearchBoxModel::new("Search models"))
                .with_empty_message("No matching models"),
            "Space search  ·  ↑/↓ select  ·  Enter apply  ·  Esc back",
        ),
        actions,
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
