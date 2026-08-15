use super::*;
use zeta_config::HookEnablement;
use zeta_config::HookEvent;
use zeta_config::HookId;
use zeta_config::HookMatcher;

fn hook(id: &str) -> HookConfig {
    HookConfig {
        id: HookId::new(id).expect("test Hook id"),
        event: HookEvent::BeforeTool,
        matcher: HookMatcher::default(),
        action: HookAction::Process {
            program: "hook-program".into(),
            args: Vec::new(),
        },
        enablement: HookEnablement::Enabled,
    }
}

fn test_workspace() -> WorkspaceRoot {
    WorkspaceRoot::open(std::env::current_dir().expect("test working directory"))
        .expect("workspace root")
}

#[test]
fn review_authority_is_bound_to_the_exact_hook_identity() {
    let workspace = test_workspace();
    let first = hook("user:hook:first");
    let second = hook("user:hook:second");

    let first_review = review_request(&first, &workspace, "hook-test-policy".into()).unwrap();
    let second_review = review_request(&second, &workspace, "hook-test-policy".into()).unwrap();
    assert_eq!(first_review.provenance().source_id(), "user:hook:first");
    assert_ne!(
        first_review.action().digest(),
        second_review.action().digest()
    );
}
