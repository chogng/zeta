mod event_source;
pub(crate) mod hyperlinks;
pub(crate) mod mouse;
pub(crate) mod screen_selection;
mod scrollback;
mod session;
mod terminal_probe;

pub(crate) use event_source::TerminalEvent;
pub(crate) use event_source::TerminalEventSource;
pub(crate) use session::TerminalSession;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ScreenMode {
    #[default]
    Fullscreen,
    Native,
}

impl ScreenMode {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Fullscreen => Self::Native,
            Self::Native => Self::Fullscreen,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Fullscreen => "fullscreen",
            Self::Native => "native",
        }
    }
}
