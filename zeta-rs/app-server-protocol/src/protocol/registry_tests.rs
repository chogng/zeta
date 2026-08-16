use super::CLIENT_METHODS;
use super::ClientRequestSerializationScope;
use super::SerializationAccess;

fn definition(method: &str) -> &'static super::ClientMethodDefinition {
    CLIENT_METHODS
        .iter()
        .find(|definition| definition.method == method)
        .unwrap()
}

#[test]
fn session_scope_uses_the_declared_session_identity() {
    let scope = definition("session/request")
        .serialization_scope(&serde_json::json!({ "sessionId": "session-1" }))
        .unwrap();

    assert_eq!(
        scope,
        Some(ClientRequestSerializationScope::Session {
            session_id: "session-1".into(),
            access: SerializationAccess::Exclusive,
        })
    );
}

#[test]
fn resource_scope_keeps_resource_families_separate() {
    let resource = definition("resource/read")
        .serialization_scope(&serde_json::json!({ "resourceId": "same" }))
        .unwrap();
    let upload = definition("attachment/upload/write")
        .serialization_scope(&serde_json::json!({ "uploadId": "same" }))
        .unwrap();

    assert_ne!(resource, upload);
}

#[test]
fn declared_key_is_required_before_dispatch() {
    assert!(
        definition("session/read")
            .serialization_scope(&serde_json::json!({}))
            .is_err()
    );
}
