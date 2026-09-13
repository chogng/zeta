use super::*;

struct StandaloneDownloader {
    template: Option<Vec<u8>>,
}

impl TokenizerAssetDownloader for StandaloneDownloader {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, LocalTokenizerError> {
        match &self.template {
            Some(template) => Ok(template.clone()),
            None => Err(LocalTokenizerError::DownloadStatus {
                url: url.into(),
                status: 404,
            }),
        }
    }
}

#[test]
fn standalone_template_overrides_the_inline_config_template() {
    let source = template_source(
        "owner/model",
        "0123456789012345678901234567890123456789",
        serde_json::to_vec(&serde_json::json!({
            "chat_template": "inline",
            "bos_token": "<s>"
        }))
        .unwrap(),
        &StandaloneDownloader {
            template: Some(b"standalone".to_vec()),
        },
    )
    .unwrap();
    let config: serde_json::Value = serde_json::from_slice(&source).unwrap();

    assert_eq!(config["chat_template"], "standalone");
    assert_eq!(config["bos_token"], "<s>");
}

#[test]
fn missing_standalone_template_preserves_named_templates_from_the_config() {
    let templates = serde_json::json!([
        {"name": "default", "template": "default"},
        {"name": "tool_use", "template": "tools"}
    ]);
    let source = template_source(
        "owner/model",
        "0123456789012345678901234567890123456789",
        serde_json::to_vec(&serde_json::json!({"chat_template": templates})).unwrap(),
        &StandaloneDownloader { template: None },
    )
    .unwrap();
    let config: serde_json::Value = serde_json::from_slice(&source).unwrap();

    assert_eq!(config["chat_template"], templates);
}
