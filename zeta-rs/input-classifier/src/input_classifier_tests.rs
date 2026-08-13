use sha2::Digest;
use sha2::Sha256;

use super::InputClassificationSource;
use super::InputRoute;
use super::MODEL_SHA256;
use super::ShellCommandEvidence;
use super::TOKENIZER_SHA256;
use super::classify_input;

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
fn natural_language_words_and_prefixes_bypass_the_model() {
    for input in ["h", "hello!", "explain", "谢谢"] {
        let classification = classify_input(input, ShellCommandEvidence::Absent).unwrap();

        assert_eq!(classification.route, InputRoute::Agent, "input: {input}");
        assert_eq!(
            classification.source,
            InputClassificationSource::NaturalLanguageHeuristic
        );
    }
}

#[test]
fn shell_keywords_syntax_and_host_evidence_bypass_the_model() {
    let cases = [
        ("echo hello", ShellCommandEvidence::Absent),
        ("missing | command", ShellCommandEvidence::Absent),
        ("ambiguous input", ShellCommandEvidence::HighConfidence),
    ];

    for (input, evidence) in cases {
        let classification = classify_input(input, evidence).unwrap();
        assert_eq!(classification.route, InputRoute::Shell, "input: {input}");
        assert_eq!(
            classification.source,
            InputClassificationSource::ShellHeuristic
        );
    }
}

#[test]
fn quoted_shell_characters_remain_available_to_the_model() {
    let classification = classify_input("explain 'a | b'", ShellCommandEvidence::Absent).unwrap();

    assert_eq!(classification.route, InputRoute::Agent);
    assert_eq!(classification.source, InputClassificationSource::Model);
}

#[test]
fn model_routes_commands_and_agent_requests() {
    let cases = [
        ("git status", InputRoute::Shell),
        ("rm -rf /tmp/demo", InputRoute::Shell),
        ("帮我总结一下这个目录", InputRoute::Agent),
        ("修复 tests", InputRoute::Agent),
        ("why did cargo test fail?", InputRoute::Agent),
        ("git status 是做什么的", InputRoute::Agent),
    ];

    for (input, expected) in cases {
        let classification = classify_input(input, ShellCommandEvidence::Absent).unwrap();
        assert_eq!(classification.route, expected, "input: {input}");
        assert_eq!(classification.source, InputClassificationSource::Model);
    }
}

#[test]
fn candle_probabilities_match_the_fp32_release_runtime() {
    let command = classify_input("git status", ShellCommandEvidence::Absent).unwrap();
    let request = classify_input("帮我总结一下这个目录", ShellCommandEvidence::Absent).unwrap();

    assert!((command.confidence - 0.992_271).abs() < 0.000_1);
    assert!((request.confidence - 0.994_042).abs() < 0.000_1);
}
