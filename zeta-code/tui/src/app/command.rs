/// A typed side-effect intent emitted by the single-writer application state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AppCommand {
    Config(crate::config::Command),
    Connectors(crate::connectors::Command),
    Dirs(crate::dirs::Command),
    Host(crate::host::Command),
    Keymap(crate::keymap::Command),
    Mcp(crate::mcp::Command),
    Models(crate::models::Command),
    Sessions(crate::sessions::Command),
    Skills(crate::skills::Command),
    Status(crate::status::Command),
    Theme(crate::theme::Command),
    Thread(crate::thread::Command),
    Quit,
    Suspend,
}

macro_rules! app_command_from {
    ($command:ty, $variant:ident) => {
        impl From<$command> for AppCommand {
            fn from(command: $command) -> Self {
                Self::$variant(command)
            }
        }
    };
}

app_command_from!(crate::config::Command, Config);
app_command_from!(crate::connectors::Command, Connectors);
app_command_from!(crate::dirs::Command, Dirs);
app_command_from!(crate::host::Command, Host);
app_command_from!(crate::keymap::Command, Keymap);
app_command_from!(crate::mcp::Command, Mcp);
app_command_from!(crate::models::Command, Models);
app_command_from!(crate::sessions::Command, Sessions);
app_command_from!(crate::skills::Command, Skills);
app_command_from!(crate::status::Command, Status);
app_command_from!(crate::theme::Command, Theme);
app_command_from!(crate::thread::Command, Thread);
