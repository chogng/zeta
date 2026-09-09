use serde::Deserialize;
use serde::Serialize;
use zeta_protocol::ModelRef;

/// Backend preferences for recommending issues that can be implemented together.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueConfig {
    #[serde(default = "enabled_by_default")]
    pub recommend_merge: bool,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub repositories: std::collections::BTreeMap<String, github::IssueWorkflow>,
    #[serde(default = "default_refresh_minutes")]
    pub auto_refresh_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis_model: Option<ModelRef>,
}

fn default_refresh_minutes() -> u32 {
    10
}

fn enabled_by_default() -> bool {
    true
}

impl Default for IssueConfig {
    fn default() -> Self {
        Self {
            recommend_merge: true,
            repositories: Default::default(),
            auto_refresh_minutes: default_refresh_minutes(),
            analysis_model: None,
        }
    }
}
