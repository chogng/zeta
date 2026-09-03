use crate::thread::composer::ChatInputMode;
use serde::Deserialize;
use serde::Serialize;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FollowUpMode {
    #[default]
    Queue,
    Steer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminalSettings {
    mouse_interactions: bool,
    follow_up_mode: FollowUpMode,
    input_mode: ChatInputMode,
}

impl TerminalSettings {
    const KEYS: [&'static str; 3] = ["mouseInteractions", "followUpMode", "inputMode"];

    pub(crate) fn from_tui(section: &FrontendConfigDto) -> Result<Self, String> {
        let defaults = serde_json::to_value(Self::default())
            .map_err(|error| format!("could not build TUI defaults: {error}"))?;
        let mut values = defaults
            .as_object()
            .cloned()
            .ok_or_else(|| "TUI defaults must serialize as an object".to_owned())?;
        for key in Self::KEYS {
            if let Some(value) = section.0.get(key) {
                values.insert(key.into(), value.clone());
            }
        }
        serde_json::from_value::<Self>(values.into())
            .map_err(|error| format!("invalid [tui] configuration: {error}"))
    }

    pub(crate) fn write_to_tui(
        self,
        section: &FrontendConfigDto,
    ) -> Result<FrontendConfigDto, String> {
        let encoded = serde_json::to_value(self)
            .map_err(|error| format!("could not encode [tui] configuration: {error}"))?;
        let fields = encoded
            .as_object()
            .ok_or_else(|| "TUI configuration must serialize as an object".to_owned())?;
        let mut values = section.0.clone();
        values.remove("dirPermissions");
        for key in Self::KEYS {
            let value = fields
                .get(key)
                .ok_or_else(|| format!("TUI configuration did not encode {key}"))?;
            values.insert(key.into(), value.clone());
        }
        Ok(FrontendConfigDto(values))
    }

    pub(crate) const fn mouse_interactions(self) -> bool {
        self.mouse_interactions
    }

    pub(crate) fn set_mouse_interactions(&mut self, enabled: bool) {
        self.mouse_interactions = enabled;
    }

    pub(crate) const fn follow_up_mode(self) -> FollowUpMode {
        self.follow_up_mode
    }

    pub(crate) fn set_follow_up_mode(&mut self, mode: FollowUpMode) {
        self.follow_up_mode = mode;
    }

    pub(crate) const fn input_mode(self) -> ChatInputMode {
        self.input_mode
    }

    pub(crate) fn set_input_mode(&mut self, mode: ChatInputMode) {
        self.input_mode = mode;
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            mouse_interactions: true,
            follow_up_mode: FollowUpMode::Queue,
            input_mode: ChatInputMode::Standard,
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
