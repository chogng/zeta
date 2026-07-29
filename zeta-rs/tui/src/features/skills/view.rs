use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
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

pub(crate) struct SkillSelectionView {
    pub(crate) model: SelectionViewModel,
    pub(crate) actions: BTreeMap<SelectionItemId, SkillSelectionAction>,
}

pub(crate) fn skills_selection_view(catalog: &SkillListResult) -> SkillSelectionView {
    let mut actions = BTreeMap::new();
    let all = catalog
        .skills
        .iter()
        .enumerate()
        .map(|(index, skill)| {
            let item_id = SelectionItemId::new(format!("skill-{index}"));
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
            SelectionItem::new(skill.id.name.as_str())
                .with_id(item_id)
                .with_description(format!(
                    "{}  ·  {}  ·  {}  ·  {}",
                    enablement_label(skill.enablement),
                    source_kind_label(skill.source_kind),
                    skill.id.source,
                    skill.description
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
            SelectionItem::new(diagnostic.subject.as_deref().unwrap_or(&diagnostic.source))
                .with_description(&diagnostic.message)
        })
        .collect::<Vec<_>>();
    let enabled_count = enabled.len();
    let disabled_count = disabled.len();
    let error_count = errors.len();

    SkillSelectionView {
        model: SelectionViewModel::new(
            "Skills",
            vec![
                SelectionTab::new(format!("All ({})", all.len()), all),
                SelectionTab::new(format!("Enabled ({enabled_count})"), enabled),
                SelectionTab::new(format!("Disabled ({disabled_count})"), disabled),
                SelectionTab::new(format!("Errors ({error_count})"), errors),
            ],
        )
        .with_search_placeholder("Search available skills")
        .with_empty_message("No matching skills")
        .with_footer_hint(
            "Type to search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Space toggle  ·  Esc back",
        ),
        actions,
    }
}

fn source_kind_label(kind: SkillSourceKindDto) -> &'static str {
    match kind {
        SkillSourceKindDto::BuiltIn => "built-in",
        SkillSourceKindDto::User => "user",
    }
}

fn enablement_label(enablement: SkillEnablementDto) -> &'static str {
    match enablement {
        SkillEnablementDto::Disabled => "disabled",
        SkillEnablementDto::Enabled => "enabled",
    }
}
