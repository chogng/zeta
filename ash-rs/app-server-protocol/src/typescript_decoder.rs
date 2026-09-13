use serde_json::Value;

pub(crate) fn generate(schema: &Value) -> String {
    let schema = serde_json::to_string(schema).expect("protocol schema must serialize as JSON");
    let schema_literal =
        serde_json::to_string(&schema).expect("protocol schema JSON must serialize as a string");
    include_str!("typescript_decoder.template.ts")
        .replace("__PROTOCOL_SCHEMA__", &schema_literal)
        .replace("\r\n", "\n")
}
