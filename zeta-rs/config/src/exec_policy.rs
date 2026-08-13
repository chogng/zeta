use crate::ConfigError;
use crate::WorkspaceId;
use serde::Deserialize;
use serde::Serialize;
use zeta_execpolicy::ExecPolicyDefault;
use zeta_execpolicy::ExecPolicyLayer;
use zeta_execpolicy::ExecPolicyLayerId;
use zeta_execpolicy::ExecPolicyLayerKind;
use zeta_execpolicy::ExecPolicyRule;
use zeta_execpolicy::ExecPolicyRuleId;
use zeta_execpolicy::ExecPolicySnapshot;

/// Durable user-owned execution-policy rules.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserExecPolicyConfig {
    #[serde(default)]
    pub rules: Vec<ExecPolicyRule>,
}

impl UserExecPolicyConfig {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        self.snapshot_layer().map(|_| ())
    }

    pub(crate) fn snapshot_layer(&self) -> Result<ExecPolicyLayer, ConfigError> {
        let layer = ExecPolicyLayer::new(
            ExecPolicyLayerId::new("user"),
            ExecPolicyLayerKind::User,
            self.rules.clone(),
        );
        ExecPolicySnapshot::new(ExecPolicyDefault::Continue, vec![layer.clone()])
            .map_err(|error| ConfigError(error.to_string()))?;
        Ok(layer)
    }

    pub(crate) fn upsert(&mut self, rule: ExecPolicyRule) {
        if let Some(existing) = self
            .rules
            .iter_mut()
            .find(|existing| existing.id() == rule.id())
        {
            *existing = rule;
        } else {
            self.rules.push(rule);
        }
    }

    pub(crate) fn remove(&mut self, rule_id: &ExecPolicyRuleId) -> bool {
        let original = self.rules.len();
        self.rules.retain(|rule| rule.id() != rule_id);
        self.rules.len() != original
    }
}

/// Workspace-authored policy restrictions preserved as untrusted configuration input.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceExecPolicyConfig {
    #[serde(default)]
    pub rules: Vec<ExecPolicyRule>,
}

impl WorkspaceExecPolicyConfig {
    pub(crate) fn snapshot_layer(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<ExecPolicyLayer, ConfigError> {
        let layer = ExecPolicyLayer::new(
            ExecPolicyLayerId::new(format!("workspace:{}", workspace_id.as_str())),
            ExecPolicyLayerKind::Workspace,
            self.rules.clone(),
        );
        ExecPolicySnapshot::new(ExecPolicyDefault::Continue, vec![layer.clone()])
            .map_err(|error| ConfigError(error.to_string()))?;
        Ok(layer)
    }
}

/// Composes trusted host/organization layers with durable User rules and restrictive Workspace
/// rules into the immutable snapshot consumed by action policy.
pub fn compose_exec_policy(
    default: ExecPolicyDefault,
    mut trusted_layers: Vec<ExecPolicyLayer>,
    user: &UserExecPolicyConfig,
    workspace: Option<(&WorkspaceId, &WorkspaceExecPolicyConfig)>,
) -> Result<ExecPolicySnapshot, ConfigError> {
    if trusted_layers.iter().any(|layer| {
        !matches!(
            layer.kind(),
            ExecPolicyLayerKind::Host | ExecPolicyLayerKind::Organization
        )
    }) {
        return Err(ConfigError(
            "trusted execution-policy inputs must be Host or Organization layers".into(),
        ));
    }
    trusted_layers.push(user.snapshot_layer()?);
    if let Some((workspace_id, workspace)) = workspace {
        trusted_layers.push(workspace.snapshot_layer(workspace_id)?);
    }
    ExecPolicySnapshot::new(default, trusted_layers).map_err(|error| ConfigError(error.to_string()))
}

#[cfg(test)]
#[path = "exec_policy_tests.rs"]
mod tests;
