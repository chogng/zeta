use crate::CodexAppServerRuntime;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

const MODEL_PAGE_LIMIT: u32 = 100;
const MAX_MODEL_PAGES: usize = 32;
const MAX_MODELS: usize = 2_048;

/// Stable provider identity used for models executed through a ChatGPT subscription.
pub const CODEX_SUBSCRIPTION_PROVIDER_ID: &str = "openai-chatgpt";

/// One redacted model advertised by the upstream Codex App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexCatalogModel {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
}

/// Failure to read or validate the upstream subscription model catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexModelCatalogError {
    message: &'static str,
}

impl fmt::Display for CodexModelCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for CodexModelCatalogError {}

/// Reads the account-filtered model catalog from the shared Codex App Server runtime.
///
/// The source exposes only opaque model IDs, display names, and the default marker. It does not
/// cache entitlement, inspect credentials, or reinterpret unavailable models as API-key models.
pub struct CodexModelCatalog {
    runtime: Arc<CodexAppServerRuntime>,
}

impl CodexModelCatalog {
    pub fn new(runtime: Arc<CodexAppServerRuntime>) -> Self {
        Self { runtime }
    }

    pub fn list(&self) -> Result<Vec<CodexCatalogModel>, CodexModelCatalogError> {
        let account = self
            .runtime
            .request("account/read", json!({ "refreshToken": false }))
            .map_err(|_| catalog_unavailable())?;
        if account
            .get("account")
            .is_none_or(|account| account.is_null())
        {
            return Ok(Vec::new());
        }
        if account.pointer("/account/type").and_then(Value::as_str) != Some("chatgpt") {
            return Ok(Vec::new());
        }
        let mut models = Vec::new();
        let mut cursor = None;
        let mut seen_cursors = BTreeSet::new();
        for _ in 0..MAX_MODEL_PAGES {
            let response = self
                .runtime
                .request(
                    "model/list",
                    json!({
                        "cursor": cursor,
                        "limit": MODEL_PAGE_LIMIT,
                        "includeHidden": false,
                    }),
                )
                .map_err(|_| catalog_unavailable())?;
            let page = response
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(incompatible_catalog)?;
            for value in page {
                let id = value
                    .get("model")
                    .and_then(Value::as_str)
                    .ok_or_else(incompatible_catalog)?;
                let display_name = value
                    .get("displayName")
                    .and_then(Value::as_str)
                    .ok_or_else(incompatible_catalog)?;
                if id.trim().is_empty() || display_name.trim().is_empty() {
                    return Err(incompatible_catalog());
                }
                if models
                    .iter()
                    .any(|model: &CodexCatalogModel| model.id == id)
                {
                    return Err(incompatible_catalog());
                }
                models.push(CodexCatalogModel {
                    id: id.into(),
                    display_name: display_name.into(),
                    is_default: value
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                });
                if models.len() > MAX_MODELS {
                    return Err(incompatible_catalog());
                }
            }
            let next = match response.get("nextCursor") {
                None | Some(Value::Null) => return Ok(models),
                Some(Value::String(next)) if !next.trim().is_empty() => next.clone(),
                _ => return Err(incompatible_catalog()),
            };
            if !seen_cursors.insert(next.clone()) {
                return Err(incompatible_catalog());
            }
            cursor = Some(next);
        }
        Err(incompatible_catalog())
    }
}

fn catalog_unavailable() -> CodexModelCatalogError {
    CodexModelCatalogError {
        message: "Codex subscription model catalog is unavailable",
    }
}

fn incompatible_catalog() -> CodexModelCatalogError {
    CodexModelCatalogError {
        message: "installed Codex App Server returned an incompatible model catalog",
    }
}
