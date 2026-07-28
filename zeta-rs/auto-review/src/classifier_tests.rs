use super::*;
use zeta_async_utils::CancellationSource;
use zeta_policy::{
    ActionClassifier, ActionDigest, ActionKind, ActionProvenance, ActionReviewRequest,
    ActionSource, Capability, CapabilityKind, CapabilitySet, ClassifierRecommendation,
    PolicyRevision, ProcessInvocationKind, ResolvedAction, SandboxCompatibility,
};

struct StaticModel(&'static str);

impl ReviewModel for StaticModel {
    fn complete(
        &self,
        _: &ReviewModelRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<String, String> {
        Ok(self.0.to_owned())
    }
}

fn request(sandbox: SandboxCompatibility) -> ActionReviewRequest {
    let capability = Capability::new(CapabilityKind::Network, "api.example.com");
    ActionReviewRequest::new(
        ResolvedAction::new(
            ActionDigest::from_canonical_bytes(b"curl api.example.com"),
            ActionKind::LocalProcess(ProcessInvocationKind::Direct),
            "call the configured API",
            CapabilitySet::new([capability]),
        ),
        ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
        sandbox,
        PolicyRevision::new("policy-7"),
    )
}

#[test]
fn binds_model_advice_to_the_host_action_and_policy_revision() {
    let model = StaticModel(
        r#"{
            "recommendation":"approve",
            "capabilities":[{"kind":"network","scope":"api.example.com"}],
            "risk":"medium",
            "user_authorization":"implicit",
            "reason":"the action requires the configured API"
        }"#,
    );
    let classifier = LlmActionClassifier::new(model, "review-prompt-3");
    let request = request(SandboxCompatibility::Unsupported {
        reason: "network is unavailable".to_owned(),
    });

    let assessment = classifier
        .classify(&request, &CancellationSource::new().token())
        .unwrap();

    assert_eq!(assessment.action_digest(), request.action().digest());
    assert_eq!(assessment.policy_revision(), request.policy_revision());
    assert_eq!(assessment.prompt_revision(), "review-prompt-3");
    assert!(matches!(
        assessment.recommendation(),
        ClassifierRecommendation::Approve { .. }
    ));
}

#[test]
fn identical_assessments_receive_the_same_audit_identity() {
    let classifier = LlmActionClassifier::new(
        StaticModel(r#"{"recommendation":"deny","reason":"unsafe"}"#),
        "review-prompt-3",
    );
    let request = request(SandboxCompatibility::Unsupported {
        reason: "network is unavailable".to_owned(),
    });

    let first = classifier
        .classify(&request, &CancellationSource::new().token())
        .unwrap();
    let second = classifier
        .classify(&request, &CancellationSource::new().token())
        .unwrap();

    assert_eq!(first.assessment_id(), second.assessment_id());
    assert_eq!(first.assessment_id().as_str().len(), 64);
}

#[test]
fn rejects_capabilities_not_present_in_the_resolved_action() {
    let model = StaticModel(
        r#"{
            "recommendation":"approve",
            "capabilities":[{"kind":"system_configuration","scope":"host"}],
            "risk":"medium",
            "user_authorization":"implicit",
            "reason":"change the host"
        }"#,
    );
    let classifier = LlmActionClassifier::new(model, "review-prompt-3");
    let request = request(SandboxCompatibility::Unsupported {
        reason: "not supported".to_owned(),
    });

    assert!(matches!(
        classifier.classify(&request, &CancellationSource::new().token()),
        Err(AutoReviewError::InvalidResponse(_))
    ));
}

#[test]
fn rejects_incomplete_approval_recommendation() {
    let model = StaticModel(r#"{"recommendation":"approve","reason":"missing fields"}"#);
    let classifier = LlmActionClassifier::new(model, "review-prompt-3");
    let request = request(SandboxCompatibility::NotApplicable {
        reason: "external mutation".to_owned(),
    });

    assert!(matches!(
        classifier.classify(&request, &CancellationSource::new().token()),
        Err(AutoReviewError::InvalidResponse(_))
    ));
}

#[test]
fn rejects_unknown_response_fields_instead_of_ignoring_them() {
    let model = StaticModel(
        r#"{
            "recommendation":"deny",
            "reason":"unsafe",
            "authorization":"granted"
        }"#,
    );
    let classifier = LlmActionClassifier::new(model, "review-prompt-3");
    let request = request(SandboxCompatibility::Unsupported {
        reason: "not supported".to_owned(),
    });

    assert!(matches!(
        classifier.classify(&request, &CancellationSource::new().token()),
        Err(AutoReviewError::InvalidResponse(_))
    ));
}

#[test]
fn cancellation_prevents_the_review_model_from_starting() {
    let classifier = LlmActionClassifier::new(
        StaticModel(r#"{"recommendation":"deny","reason":"unused"}"#),
        "review-prompt-3",
    );
    let source = CancellationSource::new();
    source.cancel();
    let request = request(SandboxCompatibility::Unsupported {
        reason: "not supported".to_owned(),
    });

    assert_eq!(
        classifier.classify(&request, &source.token()),
        Err(AutoReviewError::Cancelled)
    );
}
