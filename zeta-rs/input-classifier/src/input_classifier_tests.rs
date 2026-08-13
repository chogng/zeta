use sha2::Digest;
use sha2::Sha256;
use std::fs;

use super::InputClassificationContext;
use super::InputClassificationSource;
use super::InputClassifier;
use super::InputConversation;
use super::InputHistoryEntry;
use super::InputRoute;
use super::MODEL_SHA256;
use super::TOKENIZER_SHA256;
use crate::classifier::classify_with_model_attempt;
use crate::model::ModelAttempt;
use crate::model::classify_with_embedded_model;

fn standalone(current_route: InputRoute) -> InputClassificationContext {
    InputClassificationContext::new(current_route, InputConversation::Standalone)
}

#[test]
fn embedded_assets_match_the_pinned_release() {
    let model = include_bytes!("../models/bert_tiny_v3_candle.onnx");
    let tokenizer = include_bytes!("../models/bert_tiny_tokenizer.json");

    assert_eq!(format!("{:x}", Sha256::digest(model)), MODEL_SHA256);
    assert_eq!(format!("{:x}", Sha256::digest(tokenizer)), TOKENIZER_SHA256);
}

#[test]
fn metadata_matches_the_runtime_contract() {
    let metadata: serde_json::Value =
        serde_json::from_slice(include_bytes!("../models/metadata.json")).unwrap();

    assert_eq!(metadata["version"], "v3");
    assert_eq!(metadata["labels"]["0"], "shell");
    assert_eq!(metadata["labels"]["1"], "ai");
    assert_eq!(
        metadata["deployment"]["onnx_variants"]["candle_fp32"]["sha256"],
        MODEL_SHA256
    );
    assert_eq!(metadata["tokenizer"]["sha256"], TOKENIZER_SHA256);
    assert_eq!(
        metadata["deployment"]["onnx_variants"]["candle_fp32"]["probability_calibration"]["temperature"],
        1.6894922825552194
    );
    assert_eq!(
        metadata["deployment"]["onnx_variants"]["candle_fp32"]["opset"],
        16
    );
}

#[test]
fn deterministic_allowlists_run_before_the_model() {
    let classifier = InputClassifier::default();
    let cases = [
        (
            "hello!",
            InputRoute::Agent,
            InputClassificationSource::NaturalLanguageOneOff,
        ),
        (
            "谢谢",
            InputRoute::Agent,
            InputClassificationSource::NaturalLanguageOneOff,
        ),
        (
            "echo hello",
            InputRoute::Shell,
            InputClassificationSource::ShellAllowlist,
        ),
        (
            "sudo apt update",
            InputRoute::Shell,
            InputClassificationSource::ShellAllowlist,
        ),
    ];

    for (input, route, source) in cases {
        let classification = classifier.classify(input, standalone(InputRoute::Agent));
        assert_eq!(classification.route, route, "input: {input}");
        assert_eq!(classification.source, source, "input: {input}");
    }
}

#[test]
fn current_agent_route_keeps_a_natural_language_lead_in_agent() {
    let classifier = InputClassifier::default();

    let classification = classifier.classify("hello there", standalone(InputRoute::Agent));

    assert_eq!(classification.route, InputRoute::Agent);
    assert_eq!(
        classification.source,
        InputClassificationSource::NaturalLanguageOneOff
    );
}

#[test]
fn short_replies_use_agent_follow_up_context() {
    let classifier = InputClassifier::default();
    let standalone = classifier.classify("continue", standalone(InputRoute::Agent));
    let classification = classifier.classify(
        "continue",
        InputClassificationContext::new(InputRoute::Shell, InputConversation::AgentFollowUp),
    );

    assert_eq!(standalone.route, InputRoute::Shell);
    assert_eq!(classification.route, InputRoute::Agent);
    assert_eq!(
        classification.source,
        InputClassificationSource::AgentFollowUp
    );
}

#[test]
fn workspace_token_semantics_short_circuit_only_clear_commands() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Justfile"), "build:\n    cargo build\n").unwrap();
    let classifier = InputClassifier::for_working_directory(root.path());

    let command = classifier.classify("just build", standalone(InputRoute::Agent));
    let question = classifier.classify("just build 为什么失败了", standalone(InputRoute::Shell));

    assert_eq!(command.route, InputRoute::Shell);
    assert_eq!(
        command.source,
        InputClassificationSource::ShellTokenHeuristic
    );
    assert_eq!(question.route, InputRoute::Agent);
    assert_eq!(question.source, InputClassificationSource::Model);
}

#[test]
fn newest_close_submission_history_runs_before_shell_and_model_detection() {
    let mut classifier = InputClassifier::default();
    classifier.replace_history([
        InputHistoryEntry::shell("echo production"),
        InputHistoryEntry::agent("echo productions"),
    ]);

    let classification = classifier.classify("echo productions", standalone(InputRoute::Shell));

    assert_eq!(classification.route, InputRoute::Agent);
    assert_eq!(
        classification.source,
        InputClassificationSource::HistoryMatch
    );
}

#[test]
fn model_routes_ambiguous_commands_and_agent_requests() {
    let classifier = InputClassifier::default();
    let cases = [
        ("rm -rf /tmp/demo", InputRoute::Shell),
        ("帮我总结一下这个目录", InputRoute::Agent),
        ("修复 tests", InputRoute::Agent),
        ("why did cargo test fail?", InputRoute::Agent),
        ("git status 是做什么的", InputRoute::Agent),
    ];

    for (input, expected) in cases {
        let classification = classifier.classify(input, standalone(InputRoute::Agent));
        assert_eq!(classification.route, expected, "input: {input}");
    }
}

#[test]
fn warp_decision_cases_remain_aligned() {
    let classifier = InputClassifier::default();
    let cases = [
        ("cargo --version", InputRoute::Shell),
        ("rvm install 3.3", InputRoute::Shell),
        ("Explain this", InputRoute::Agent),
        ("fix this", InputRoute::Agent),
        ("What went wrong?", InputRoute::Agent),
        ("What's the reason", InputRoute::Agent),
        ("The message is \"utils::future ... ok\"", InputRoute::Agent),
        ("The type is \"<>\"", InputRoute::Agent),
    ];

    for (input, expected) in cases {
        let classification = classifier.classify(input, standalone(InputRoute::Shell));
        assert_eq!(classification.route, expected, "input: {input}");
    }
}

#[test]
fn ordinary_model_failure_preserves_the_current_route() {
    let classifier = InputClassifier::default();
    let classification = classify_with_model_attempt(
        &classifier,
        "help migrate database",
        standalone(InputRoute::Shell),
        ModelAttempt::Failed,
    );

    assert_eq!(classification.route, InputRoute::Shell);
    assert_eq!(classification.confidence, 0.0);
    assert_eq!(
        classification.source,
        InputClassificationSource::CurrentRouteFallback
    );
}

#[test]
fn model_panic_uses_the_dictionary_heuristic() {
    let classifier = InputClassifier::default();
    let classification = classify_with_model_attempt(
        &classifier,
        "fix this",
        standalone(InputRoute::Shell),
        ModelAttempt::Panicked,
    );

    assert_eq!(classification.route, InputRoute::Agent);
    assert_eq!(
        classification.source,
        InputClassificationSource::HeuristicFallback
    );
}

#[test]
fn unavailable_model_uses_the_dictionary_heuristic() {
    let classifier = InputClassifier::default();
    let classification = classify_with_model_attempt(
        &classifier,
        "fix this",
        standalone(InputRoute::Shell),
        ModelAttempt::Unavailable,
    );

    assert_eq!(classification.route, InputRoute::Agent);
    assert_eq!(
        classification.source,
        InputClassificationSource::HeuristicFallback
    );
}

#[test]
fn candle_probabilities_match_the_fp32_release_runtime() {
    let ModelAttempt::Classified(command) = classify_with_embedded_model("git status") else {
        panic!("model should classify command");
    };
    let ModelAttempt::Classified(request) = classify_with_embedded_model("帮我总结一下这个目录")
    else {
        panic!("model should classify request");
    };

    assert!((command.confidence - 0.992_271).abs() < 0.000_1);
    assert!((request.confidence - 0.994_042).abs() < 0.000_1);
}
