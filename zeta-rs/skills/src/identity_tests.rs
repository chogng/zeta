use super::*;

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
fn source_and_content_identities_validate_during_deserialization() {
    let source = SkillSourceId::new("user:skill-source:personal").unwrap();
    assert_eq!(source.as_str(), "user:skill-source:personal");
    assert!(SkillSourceId::new("personal").is_err());
    assert!(serde_yaml::from_str::<SkillName>("Review").is_err());
    assert!(ContentDigest::new(format!("sha256:{}", "0".repeat(64))).is_ok());
    assert!(ContentDigest::new(format!("sha256:{}", "A".repeat(64))).is_err());
}
