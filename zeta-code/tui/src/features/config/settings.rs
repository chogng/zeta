use crate::components::chat_input::ChatInputMode;
use serde::Deserialize;
use serde::Serialize;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;
use zeta_app_server_protocol::protocol::environment::PermissionDto;

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
    dir_permissions: DirPermissionDefaults,
}

impl TerminalSettings {
    const KEYS: [&'static str; 4] = [
        "mouseInteractions",
        "followUpMode",
        "inputMode",
        "dirPermissions",
    ];

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
            .map_err(|error| format!("invalid [tui] configuration: {error}"))?
            .validate()
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

    pub(crate) fn dir_permissions(self) -> Vec<PermissionDto> {
        self.dir_permissions.permissions()
    }

    pub(crate) fn set_dir_permissions(&mut self, permissions: &[PermissionDto]) {
        self.dir_permissions = DirPermissionDefaults::from_permissions(permissions);
    }

    pub(crate) fn validate(self) -> Result<Self, String> {
        let permissions = self.dir_permissions();
        if !permissions.is_empty() && !permissions.contains(&PermissionDto::ReadFiles) {
            return Err(
                "directory defaults require Read files when another permission is enabled".into(),
            );
        }
        Ok(self)
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            mouse_interactions: true,
            follow_up_mode: FollowUpMode::Queue,
            input_mode: ChatInputMode::Standard,
            dir_permissions: DirPermissionDefaults::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct DirPermissionDefaults {
    read_files: bool,
    write_files: bool,
    execute_commands: bool,
    watch_files: bool,
    browse_files: bool,
    search_files: bool,
    load_instructions: bool,
    load_config: bool,
    discover_skills: bool,
    discover_mcp: bool,
    use_language_services: bool,
    discover_hooks: bool,
    discover_plugins: bool,
    inspect_repository: bool,
    mutate_repository: bool,
}

impl DirPermissionDefaults {
    fn permissions(self) -> Vec<PermissionDto> {
        use PermissionDto as Permission;

        [
            (self.read_files, Permission::ReadFiles),
            (self.write_files, Permission::WriteFiles),
            (self.execute_commands, Permission::ExecuteCommands),
            (self.watch_files, Permission::WatchFiles),
            (self.browse_files, Permission::BrowseFiles),
            (self.search_files, Permission::SearchFiles),
            (self.load_instructions, Permission::LoadInstructions),
            (self.load_config, Permission::LoadConfig),
            (self.discover_skills, Permission::DiscoverSkills),
            (self.discover_mcp, Permission::DiscoverMcp),
            (self.use_language_services, Permission::UseLanguageServices),
            (self.discover_hooks, Permission::DiscoverHooks),
            (self.discover_plugins, Permission::DiscoverPlugins),
            (self.inspect_repository, Permission::InspectRepository),
            (self.mutate_repository, Permission::MutateRepository),
        ]
        .into_iter()
        .filter_map(|(enabled, permission)| enabled.then_some(permission))
        .collect()
    }

    fn from_permissions(permissions: &[PermissionDto]) -> Self {
        use PermissionDto as Permission;

        Self {
            read_files: permissions.contains(&Permission::ReadFiles),
            write_files: permissions.contains(&Permission::WriteFiles),
            execute_commands: permissions.contains(&Permission::ExecuteCommands),
            watch_files: permissions.contains(&Permission::WatchFiles),
            browse_files: permissions.contains(&Permission::BrowseFiles),
            search_files: permissions.contains(&Permission::SearchFiles),
            load_instructions: permissions.contains(&Permission::LoadInstructions),
            load_config: permissions.contains(&Permission::LoadConfig),
            discover_skills: permissions.contains(&Permission::DiscoverSkills),
            discover_mcp: permissions.contains(&Permission::DiscoverMcp),
            use_language_services: permissions.contains(&Permission::UseLanguageServices),
            discover_hooks: permissions.contains(&Permission::DiscoverHooks),
            discover_plugins: permissions.contains(&Permission::DiscoverPlugins),
            inspect_repository: permissions.contains(&Permission::InspectRepository),
            mutate_repository: permissions.contains(&Permission::MutateRepository),
        }
    }
}

impl Default for DirPermissionDefaults {
    fn default() -> Self {
        Self {
            read_files: true,
            write_files: true,
            execute_commands: false,
            watch_files: false,
            browse_files: false,
            search_files: false,
            load_instructions: false,
            load_config: false,
            discover_skills: false,
            discover_mcp: false,
            use_language_services: false,
            discover_hooks: false,
            discover_plugins: false,
            inspect_repository: false,
            mutate_repository: false,
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
