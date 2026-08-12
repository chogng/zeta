use super::{ContentDigest, SkillRef, SkillVersionSelector};
use super::{InvalidSkillName, SkillId, SkillName, SkillSourceId};

#[test]
fn skill_names_follow_the_agent_skills_contract() {
    assert_eq!(
        SkillName::new("code-review").unwrap().as_str(),
        "code-review"
    );
    for invalid in [
        "",
        "Review",
        "-review",
        "review-",
        "code--review",
        "code_review",
        "技能",
    ] {
        assert!(SkillName::new(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        SkillName::new("a".repeat(65)),
        Err(InvalidSkillName::TooLong)
    );
}

#[test]
fn source_qualified_identity_round_trips() {
    let source = SkillSourceId::new("user:skill-source:personal").unwrap();
    assert!(source.belongs_to_namespace("user"));
    assert!(!source.belongs_to_namespace("workspace:project"));
    assert!(SkillSourceId::new("personal").is_err());

    let skill = SkillId::new(source, SkillName::new("review").unwrap());
    let encoded = serde_json::to_string(&skill).unwrap();
    assert_eq!(serde_json::from_str::<SkillId>(&encoded).unwrap(), skill);
}

#[test]
fn skill_refs_bind_catalog_identity_without_a_filesystem_path() {
    let id = SkillId::new(
        SkillSourceId::new("user:skill-source:personal").unwrap(),
        SkillName::new("review").unwrap(),
    );
    let digest = ContentDigest::sha256(b"exact skill");
    let selected = SkillRef::pinned(id, digest.clone());
    let encoded = serde_json::to_value(&selected).unwrap();

    assert!(encoded.get("path").is_none());
    assert!(matches!(
        selected.version,
        SkillVersionSelector::PinnedDigest { digest: selected } if selected == digest
    ));
}
