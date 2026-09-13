use super::AppServerError;
use super::AppServerErrorName;

#[test]
fn error_kind_is_structured_separately_from_diagnostic_message() {
    let error = AppServerError::new(-32013, AppServerErrorName::ResourceNotFound);

    assert_eq!(
        serde_json::to_value(error).unwrap(),
        serde_json::json!({
            "code": -32013,
            "message": "ResourceNotFound",
            "data": { "kind": "ResourceNotFound" },
        })
    );
}
