use super::SkillSelector;
use super::SkillSelectorItem;
use crate::components::composer::editor::TextArea;
use zeta_protocol::ContentDigest;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;

#[test]
fn dollar_token_selects_and_binds_an_exact_skill() {
    let skill = skill_ref("commit");
    let mut selector = SkillSelector::default();
    selector.replace_catalog(vec![SkillSelectorItem::new(
        "commit".into(),
        "draft a commit message".into(),
        skill.clone(),
    )]);
    let mut textarea = TextArea::new();
    textarea.insert_text("use $com");
    selector.sync_textarea(&textarea);

    assert_eq!(selector.view().unwrap().items[0].name(), "commit");
    assert!(selector.complete_selected(&mut textarea));
    assert_eq!(textarea.text(), "use $commit ");
    let (element_id, _) = textarea.elements().next().unwrap();
    assert_eq!(selector.skill_for(element_id), Some(&skill));
}

#[test]
fn at_token_does_not_open_the_skill_selector() {
    let mut selector = SkillSelector::default();
    selector.replace_catalog(vec![SkillSelectorItem::new(
        "commit".into(),
        "draft a commit message".into(),
        skill_ref("commit"),
    )]);
    let mut textarea = TextArea::new();
    textarea.insert_text("@commit");
    selector.sync_textarea(&textarea);

    assert_eq!(selector.view(), None);
}

fn skill_ref(name: &str) -> SkillRef {
    SkillRef::pinned(
        SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new(name).unwrap(),
        ),
        ContentDigest::sha256(name.as_bytes()),
    )
}
