use super::ConfigChoices;
use super::ConfigSelectionAction;
use crate::keymap::bindings;
use crate::nls;
use crate::nls::Language;
use crate::nls::Message;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::search_box::SearchBoxModel;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::issues::IssueConfigDto;
use zeta_app_server_protocol::protocol::model::ModelListResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IssueConfigEdit {
    pub(crate) expected_revision: u64,
    pub(crate) config: IssueConfigDto,
}

pub(super) fn model_choices(
    config: &ConfigReadResult,
    catalog: ModelListResult,
    language: Language,
) -> ConfigChoices {
    let mut actions = BTreeMap::new();
    let clear_id = ListSelectionItemId::new("issue-model-clear");
    let mut cleared = config.issues.clone();
    cleared.analysis_model = None;
    actions.insert(
        clear_id.clone(),
        ConfigSelectionAction::SetIssues(IssueConfigEdit {
            expected_revision: config.revision,
            config: cleared,
        }),
    );
    let mut items = vec![
        ListSelectionItem::new(nls::text(language, Message::ConfigIssueModelClear))
            .with_id(clear_id),
    ];
    let mut selected = 0;
    for entry in catalog
        .models
        .into_iter()
        .filter(|entry| config.providers.contains_key(entry.model.provider.as_str()))
    {
        let model = ModelRefDto {
            provider: entry.model.provider.to_string(),
            model: entry.model.model.to_string(),
        };
        let current = config.issues.analysis_model.as_ref() == Some(&model);
        if current {
            selected = items.len();
        }
        let id = ListSelectionItemId::new(format!("issue-model-{}", items.len()));
        let mut next = config.issues.clone();
        next.analysis_model = Some(model.clone());
        actions.insert(
            id.clone(),
            ConfigSelectionAction::SetIssues(IssueConfigEdit {
                expected_revision: config.revision,
                config: next,
            }),
        );
        items.push(
            ListSelectionItem::new(format!(
                "{}{}",
                entry.display_name,
                if current { " ✓" } else { "" }
            ))
            .with_id(id)
            .with_description(format!("{}/{}", model.provider, model.model)),
        );
    }
    if items.len() == 1 {
        items.push(ListSelectionItem::new(nls::text(
            language,
            Message::ConfigIssueModelEmpty,
        )));
    }
    ConfigChoices {
        model: ListSelectionModel::new(
            nls::text(language, Message::ConfigIssueModelPicker),
            vec![ListSelectionGroup::new(
                nls::text(language, Message::ConfigIssues),
                items,
            )],
        )
        .without_tab_bar()
        .with_activation(bindings::MODEL_APPLY)
        .with_initial_selected(selected)
        .with_search(SearchBoxModel::new(nls::text(
            language,
            Message::ConfigIssueModelSearch,
        ))),
        actions,
    }
}

#[cfg(test)]
#[path = "issues_tests.rs"]
mod tests;
