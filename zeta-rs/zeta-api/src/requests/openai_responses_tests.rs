use super::*;
use crate::Message;

#[test]
fn stable_prefix_breakpoint_precedes_the_changing_environment_suffix() {
    let mut request = ModelRequest::text("stable inherited history".repeat(500));
    request.prompt_cache_key = Some("session".into());
    request.input.push(InputItem::Message(Message::text(
        MessageRole::User,
        "branch environment",
    )));
    let original = request.clone();
    let first = build_request("gpt-5.6-sol", &request).unwrap();
    request.input[1] =
        InputItem::Message(Message::text(MessageRole::User, "different environment"));
    let second = build_request("gpt-5.6-sol", &request).unwrap();
    assert_eq!(first["input"][0], second["input"][0]);
    assert_eq!(
        first["input"][0]["content"][0]["prompt_cache_breakpoint"],
        json!({"mode":"explicit"})
    );
    assert!(
        second["input"][1]["content"][0]
            .get("prompt_cache_breakpoint")
            .is_none()
    );
    assert_eq!(first["prompt_cache_key"], second["prompt_cache_key"]);
    assert_eq!(original.input[0], request.input[0]);
    assert!(
        build_count_request("gpt-5.6-sol", &request).unwrap()["input"][0]["content"][0]
            .get("prompt_cache_breakpoint")
            .is_none()
    );
}

#[test]
fn older_and_unrecognized_model_families_keep_automatic_caching() {
    for model in ["gpt-5.5", "gpt-4.1", "other-model", "gpt-invalid"] {
        let request = build_request(model, &ModelRequest::text("history")).unwrap();
        assert!(
            request["input"][0]["content"][0]
                .get("prompt_cache_breakpoint")
                .is_none()
        );
    }
    assert_eq!(cache_support("gpt-6-astra"), CacheSupport::Breakpoints);
}
