use serde_json::Value;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;

const CONFIG_KEY: &str = "statusLine";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusLineItem {
    Permissions,
    Model,
    CacheHitRate,
    ReferenceCost,
    GitBranch,
    GitChanges,
}

impl StatusLineItem {
    pub(crate) const ALL: [Self; 6] = [
        Self::Permissions,
        Self::Model,
        Self::CacheHitRate,
        Self::ReferenceCost,
        Self::GitBranch,
        Self::GitChanges,
    ];

    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "permissions" => Some(Self::Permissions),
            "model" => Some(Self::Model),
            "cache-hit-rate" => Some(Self::CacheHitRate),
            "reference-cost" => Some(Self::ReferenceCost),
            "git-branch" => Some(Self::GitBranch),
            "git-changes" => Some(Self::GitChanges),
            _ => None,
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Permissions => "permissions",
            Self::Model => "model",
            Self::CacheHitRate => "cache-hit-rate",
            Self::ReferenceCost => "reference-cost",
            Self::GitBranch => "git-branch",
            Self::GitChanges => "git-changes",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Permissions => "Permissions",
            Self::Model => "Model",
            Self::CacheHitRate => "Cache hit rate",
            Self::ReferenceCost => "Reference cost",
            Self::GitBranch => "Git branch",
            Self::GitChanges => "Git changes",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Permissions => "Current permission mode",
            Self::Model => "Configured model",
            Self::CacheHitRate => "Cached input as a share of total input",
            Self::ReferenceCost => "Current Thread accumulated reference cost",
            Self::GitBranch => "Current Git branch",
            Self::GitChanges => "Working tree changes",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatusLineSettings {
    items: Vec<StatusLineItem>,
}

impl StatusLineSettings {
    pub(crate) fn from_tui(section: &FrontendConfigDto) -> Result<Self, String> {
        let Some(value) = section.0.get(CONFIG_KEY) else {
            return Ok(Self::default());
        };
        let values = value.as_array().ok_or_else(|| {
            "invalid [tui].statusLine: expected an array of item names".to_owned()
        })?;
        let mut items = Vec::with_capacity(values.len());
        for value in values {
            let id = value.as_str().ok_or_else(|| {
                "invalid [tui].statusLine: every item name must be a string".to_owned()
            })?;
            let item = StatusLineItem::from_id(id)
                .ok_or_else(|| format!("invalid [tui].statusLine item `{id}`"))?;
            if items.contains(&item) {
                return Err(format!("invalid [tui].statusLine: duplicate item `{id}`"));
            }
            items.push(item);
        }
        Ok(Self { items })
    }

    pub(crate) fn write_to_tui(&self, section: &FrontendConfigDto) -> FrontendConfigDto {
        let mut values = section.0.clone();
        values.insert(
            CONFIG_KEY.into(),
            Value::Array(
                self.items
                    .iter()
                    .map(|item| Value::String(item.id().into()))
                    .collect(),
            ),
        );
        FrontendConfigDto(values)
    }

    pub(crate) fn enabled(&self, item: StatusLineItem) -> bool {
        self.items.contains(&item)
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = StatusLineItem> + '_ {
        self.items.iter().copied()
    }

    pub(crate) fn set(&mut self, item: StatusLineItem, enabled: bool) {
        if enabled {
            if !self.items.contains(&item) {
                self.items.push(item);
            }
        } else {
            self.items.retain(|candidate| *candidate != item);
        }
    }
}

impl Default for StatusLineSettings {
    fn default() -> Self {
        Self {
            items: vec![
                StatusLineItem::Permissions,
                StatusLineItem::Model,
                StatusLineItem::GitBranch,
                StatusLineItem::GitChanges,
            ],
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
