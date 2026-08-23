use super::*;

const BUILT_IN_PROMPTS: &[PromptArtifact] = &[
    SYSTEM_PROMPT,
    COMPACTION_PROMPT,
    GOALS_PROMPT,
    REVIEW_PROMPT,
];

#[test]
fn built_in_prompts_have_non_empty_stable_metadata() {
    for prompt in BUILT_IN_PROMPTS {
        assert!(!prompt.id().is_empty());
        assert!(!prompt.revision().is_empty());
        assert!(!prompt.body().trim().is_empty());
        assert!(prompt.body().ends_with('\n'));
    }
}

#[test]
fn built_in_prompts_have_unique_identity_and_revision() {
    for (index, prompt) in BUILT_IN_PROMPTS.iter().enumerate() {
        for other in BUILT_IN_PROMPTS.iter().skip(index + 1) {
            assert_ne!(prompt.id(), other.id());
            assert_ne!(prompt.revision(), other.revision());
        }
    }
}

#[test]
fn prompt_categories_match_their_public_assets() {
    assert_eq!(SYSTEM_PROMPT.category(), PromptCategory::System);
    assert_eq!(COMPACTION_PROMPT.category(), PromptCategory::Compaction);
    assert_eq!(GOALS_PROMPT.category(), PromptCategory::Goals);
    assert_eq!(REVIEW_PROMPT.category(), PromptCategory::Review);
}

#[test]
fn system_prompt_prefers_apply_patch_and_scopes_exact_edit() {
    assert_eq!(SYSTEM_PROMPT.revision(), "system-v4");
    assert!(
        SYSTEM_PROMPT
            .body()
            .contains("Use apply_patch as the default editing tool")
    );
    assert!(
        SYSTEM_PROMPT
            .body()
            .contains("Use edit for one small exact replacement")
    );
    assert!(SYSTEM_PROMPT.body().contains("not a transaction"));
}
