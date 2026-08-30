use super::SkillSelector;
use super::SkillSelectorItem;
use zeta_protocol::ContentDigest;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;

#[test]
fn dollar_token_returns_an_exact_skill_completion() {
    let skill = skill_ref("commit");
    let mut selector = SkillSelector::default();
    selector.replace_catalog(vec![SkillSelectorItem::new(
        "commit".into(),
        "draft a commit message".into(),
        skill.clone(),
    )]);
    selector.sync("use $com", "use $com".len());

    assert_eq!(selector.view().unwrap().items[0].name(), "commit");
    let completion = selector.complete_selected().unwrap();
    assert_eq!(completion.range, 4..8);
    assert_eq!(completion.value, "$commit");
    assert_eq!(completion.skill, skill);
}

#[test]
fn at_token_does_not_open_the_skill_selector() {
    let mut selector = SkillSelector::default();
    selector.replace_catalog(vec![SkillSelectorItem::new(
        "commit".into(),
        "draft a commit message".into(),
        skill_ref("commit"),
    )]);
    selector.sync("@commit", "@commit".len());

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
