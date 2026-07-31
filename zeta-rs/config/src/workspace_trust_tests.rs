use super::*;

fn identity() -> WorkspaceTrustId {
    format!("sha256:{}", "ab".repeat(32)).parse().unwrap()
}

#[test]
fn missing_workspace_decisions_fail_closed() {
    assert_eq!(
        WorkspaceTrustConfig::default().decision_for(&identity()),
        WorkspaceTrustDecision::Restricted
    );
}

#[test]
fn persisted_trust_resolves_as_an_explicit_user_decision() {
    let workspace = identity();
    let config = WorkspaceTrustConfig {
        roots: BTreeMap::from([(workspace.clone(), WorkspaceTrustSetting::Trusted)]),
    };

    assert_eq!(
        config.decision_for(&workspace),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision)
    );
}
