use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::skills::{
    SkillEnablementDto, SkillListResult, SkillSourceKindDto,
};
use zeta_protocol::SkillId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SkillSelectionAction {
    SetEnablement {
        skill_id: SkillId,
        enablement: SkillEnablementDto,
    },
}

pub(crate) struct SkillPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, SkillSelectionAction>,
}

pub(crate) fn skills_pane_spec(catalog: &SkillListResult) -> SkillPaneSpec {
    let mut actions = BTreeMap::new();
    let all = catalog
        .skills
        .iter()
        .enumerate()
        .map(|(index, skill)| {
            let item_id = ListSelectionItemId::new(format!("skill-{index}"));
            ListSelectionItem::new(skill.id.name.as_str())
                .with_id(item_id)
                .with_description(format!(
                    "{}  ·  {}  ·  {}  ·  {}",
                    enablement_label(skill.enablement),
                    source_kind_label(skill.source_kind),
                    skill.id.source,
                    skill.description,
                ))
        })
        .collect::<Vec<_>>();
    let enabled = all
        .iter()
        .zip(&catalog.skills)
        .filter(|(_, skill)| skill.enablement == SkillEnablementDto::Enabled)
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();
    let disabled = all
        .iter()
        .zip(&catalog.skills)
        .filter(|(_, skill)| skill.enablement == SkillEnablementDto::Disabled)
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();
    let errors = catalog
        .diagnostics
        .iter()
        .map(|diagnostic| {
            ListSelectionItem::new(diagnostic.subject.as_deref().unwrap_or(&diagnostic.source))
                .with_description(&diagnostic.message)
        })
        .collect::<Vec<_>>();
    let manage = catalog
        .skills
        .iter()
        .enumerate()
        .map(|(index, skill)| {
            let item_id = ListSelectionItemId::new(format!("manage-skill-{index}"));
            let enablement = match skill.enablement {
                SkillEnablementDto::Disabled => SkillEnablementDto::Enabled,
                SkillEnablementDto::Enabled => SkillEnablementDto::Disabled,
            };
            actions.insert(
                item_id.clone(),
                SkillSelectionAction::SetEnablement {
                    skill_id: skill.id.clone(),
                    enablement,
                },
            );
            ListSelectionItem::new(skill.id.name.as_str())
                .with_id(item_id)
                .with_description(format!(
                    "{} → {}  ·  {}",
                    enablement_label(skill.enablement),
                    enablement_label(enablement),
                    skill.id.source,
                ))
        })
        .collect::<Vec<_>>();
    let enabled_count = enabled.len();
    let disabled_count = disabled.len();
    let error_count = errors.len();

    SkillPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Skills",
                vec![
                    ListSelectionGroup::new(format!("All ({})", all.len()), all),
                    ListSelectionGroup::new(format!("Enabled ({enabled_count})"), enabled),
                    ListSelectionGroup::new(format!("Disabled ({disabled_count})"), disabled),
                    ListSelectionGroup::new("Manage", manage),
                    ListSelectionGroup::new(format!("Errors ({error_count})"), errors),
                ],
            )
            .with_search(SearchBoxModel::new("Search available skills"))
            .with_empty_message("No matching skills"),
            "↑/↓ focus  ·  ←/→ or Tab/Shift-Tab tabs  ·  Enter select  ·  Esc back",
        ),
        actions,
    }
}

fn source_kind_label(kind: SkillSourceKindDto) -> &'static str {
    match kind {
        SkillSourceKindDto::BuiltIn => "built-in",
        SkillSourceKindDto::User => "user",
        SkillSourceKindDto::Directory => "directory",
        SkillSourceKindDto::Plugin => "plugin",
        SkillSourceKindDto::Marketplace => "marketplace",
    }
}

fn enablement_label(enablement: SkillEnablementDto) -> &'static str {
    match enablement {
        SkillEnablementDto::Disabled => "disabled",
        SkillEnablementDto::Enabled => "enabled",
    }
}
