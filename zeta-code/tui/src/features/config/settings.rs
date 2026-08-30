use serde::Deserialize;
use serde::Serialize;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;

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
    additional_directory_permissions: AdditionalDirectoryPermissionDefaults,
}

impl TerminalSettings {
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

    pub(crate) fn additional_directory_permissions(
        self,
    ) -> Vec<WorkspaceAdditionalDirectoryPermissionDto> {
        self.additional_directory_permissions.permissions()
    }

    pub(crate) fn set_additional_directory_permissions(
        &mut self,
        permissions: &[WorkspaceAdditionalDirectoryPermissionDto],
    ) {
        self.additional_directory_permissions =
            AdditionalDirectoryPermissionDefaults::from_permissions(permissions);
    }

    pub(crate) fn validate(self) -> Result<Self, String> {
        let permissions = self.additional_directory_permissions();
        if !permissions.is_empty()
            && !permissions.contains(&WorkspaceAdditionalDirectoryPermissionDto::ReadFiles)
        {
            return Err(
                "additional-directory defaults require Read files when another permission is enabled"
                    .into(),
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
            additional_directory_permissions: AdditionalDirectoryPermissionDefaults::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct AdditionalDirectoryPermissionDefaults {
    read_files: bool,
    write_files: bool,
    execute_commands: bool,
    watch_file_changes: bool,
    use_workspace_files: bool,
    use_workspace_search: bool,
    load_instructions_and_agents: bool,
    discover_skills: bool,
    discover_mcp: bool,
    use_language_services: bool,
    discover_hooks: bool,
    discover_plugins: bool,
}

impl AdditionalDirectoryPermissionDefaults {
    fn permissions(self) -> Vec<WorkspaceAdditionalDirectoryPermissionDto> {
        use WorkspaceAdditionalDirectoryPermissionDto as Permission;

        [
            (self.read_files, Permission::ReadFiles),
            (self.write_files, Permission::WriteFiles),
            (self.execute_commands, Permission::ExecuteCommands),
            (self.watch_file_changes, Permission::WatchFileChanges),
            (self.use_workspace_files, Permission::UseWorkspaceFiles),
            (self.use_workspace_search, Permission::UseWorkspaceSearch),
            (
                self.load_instructions_and_agents,
                Permission::LoadInstructionsAndAgents,
            ),
            (self.discover_skills, Permission::DiscoverSkills),
            (self.discover_mcp, Permission::DiscoverMcp),
            (self.use_language_services, Permission::UseLanguageServices),
            (self.discover_hooks, Permission::DiscoverHooks),
            (self.discover_plugins, Permission::DiscoverPlugins),
        ]
        .into_iter()
        .filter_map(|(enabled, permission)| enabled.then_some(permission))
        .collect()
    }

    fn from_permissions(permissions: &[WorkspaceAdditionalDirectoryPermissionDto]) -> Self {
        use WorkspaceAdditionalDirectoryPermissionDto as Permission;

        Self {
            read_files: permissions.contains(&Permission::ReadFiles),
            write_files: permissions.contains(&Permission::WriteFiles),
            execute_commands: permissions.contains(&Permission::ExecuteCommands),
            watch_file_changes: permissions.contains(&Permission::WatchFileChanges),
            use_workspace_files: permissions.contains(&Permission::UseWorkspaceFiles),
            use_workspace_search: permissions.contains(&Permission::UseWorkspaceSearch),
            load_instructions_and_agents: permissions
                .contains(&Permission::LoadInstructionsAndAgents),
            discover_skills: permissions.contains(&Permission::DiscoverSkills),
            discover_mcp: permissions.contains(&Permission::DiscoverMcp),
            use_language_services: permissions.contains(&Permission::UseLanguageServices),
            discover_hooks: permissions.contains(&Permission::DiscoverHooks),
            discover_plugins: permissions.contains(&Permission::DiscoverPlugins),
        }
    }
}

impl Default for AdditionalDirectoryPermissionDefaults {
    fn default() -> Self {
        Self {
            read_files: true,
            write_files: true,
            execute_commands: false,
            watch_file_changes: false,
            use_workspace_files: false,
            use_workspace_search: false,
            load_instructions_and_agents: false,
            discover_skills: false,
            discover_mcp: false,
            use_language_services: false,
            discover_hooks: false,
            discover_plugins: false,
        }
    }
}
