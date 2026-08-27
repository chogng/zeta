use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminalSettings {
    mouse_interactions: bool,
}

impl TerminalSettings {
    pub(crate) const fn mouse_interactions(self) -> bool {
        self.mouse_interactions
    }

    pub(crate) fn set_mouse_interactions(&mut self, enabled: bool) {
        self.mouse_interactions = enabled;
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            mouse_interactions: true,
        }
    }
}
