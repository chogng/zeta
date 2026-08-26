use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionEntry;
use zeta_remote_connections::RemoteConnectionName;
use zeta_ui::Rect;
use zeta_ui::ScrollAxis;
use zeta_ui::ScrollCommand;
use zeta_ui::ScrollMetrics;
use zeta_ui::ScrollState;
use zeta_ui::TextInput;
use zeta_ui::TextInputCommand;
use zeta_ui::TextInputCompositionEvent;
use zui::ui::ElementId;

#[path = "remote_connection_manager_launch.rs"]
mod launch;

const REMOTE_CONNECTION_MANAGER_SCOPE: u32 = 10;
pub(crate) const REMOTE_CONNECTION_MANAGER: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 1);
pub(crate) const REMOTE_CONNECTION_MANAGER_CLOSE: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 2);
pub(crate) const REMOTE_CONNECTION_MANAGER_NEW: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 3);
pub(crate) const REMOTE_CONNECTION_MANAGER_NAME: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 4);
pub(crate) const REMOTE_CONNECTION_MANAGER_HOST: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 5);
pub(crate) const REMOTE_CONNECTION_MANAGER_WORKSPACE: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 6);
pub(crate) const REMOTE_CONNECTION_MANAGER_SAVE: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 7);
pub(crate) const REMOTE_CONNECTION_MANAGER_DELETE: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 8);
pub(crate) const REMOTE_CONNECTION_MANAGER_CONNECT: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 9);
pub(crate) const REMOTE_CONNECTION_MANAGER_LIST: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 10);
pub(crate) const REMOTE_CONNECTION_MANAGER_STATUS: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_MANAGER_SCOPE, 11);
const FIRST_REMOTE_CONNECTION_MANAGER_ITEM: u32 = 16;
pub(crate) const REMOTE_CONNECTION_MANAGER_ITEM_HEIGHT: f32 = 34.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RemoteConnectionManagerField {
    Name,
    Host,
    Workspace,
}

impl RemoteConnectionManagerField {
    pub(crate) const fn element_id(self) -> ElementId {
        match self {
            Self::Name => REMOTE_CONNECTION_MANAGER_NAME,
            Self::Host => REMOTE_CONNECTION_MANAGER_HOST,
            Self::Workspace => REMOTE_CONNECTION_MANAGER_WORKSPACE,
        }
    }

    pub(crate) fn from_element_id(id: ElementId) -> Option<Self> {
        [Self::Name, Self::Host, Self::Workspace]
            .into_iter()
            .find(|field| field.element_id() == id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RemoteConnectionSaveRequest {
    Create(RemoteConnectionEntry),
    Update {
        original: RemoteConnectionName,
        entry: RemoteConnectionEntry,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManagerStatus {
    message: String,
    error: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct OpenRemoteConnectionManager {
    connections: Vec<RemoteConnectionEntry>,
    restore_focus: Option<ElementId>,
    original: Option<RemoteConnectionName>,
    dirty: bool,
    launching: Option<RemoteConnectionName>,
    delete_confirmation: Option<RemoteConnectionName>,
    status: Option<ManagerStatus>,
}

/// Product-owned Native editor for the shared credential-free Remote target catalog.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RemoteConnectionManagerState {
    open: Option<OpenRemoteConnectionManager>,
    name: TextInput,
    host: TextInput,
    workspace: TextInput,
    scroll: ScrollState,
}

impl RemoteConnectionManagerState {
    pub(crate) fn open(
        &mut self,
        mut connections: Vec<RemoteConnectionEntry>,
        restore_focus: Option<ElementId>,
    ) {
        connections.sort_by(|left, right| left.name().cmp(right.name()));
        self.open = Some(OpenRemoteConnectionManager {
            connections,
            restore_focus,
            original: None,
            dirty: false,
            launching: None,
            delete_confirmation: None,
            status: None,
        });
        self.scroll = ScrollState::default();
        if self.connections().is_empty() {
            self.load_new_draft();
        } else {
            self.load_connection(0);
        }
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub(crate) fn dismiss(&mut self) -> Option<ElementId> {
        self.cancel_compositions_except(None);
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub(crate) fn connections(&self) -> &[RemoteConnectionEntry] {
        self.open
            .as_ref()
            .map(|open| open.connections.as_slice())
            .unwrap_or_default()
    }

    pub(crate) fn selected_name(&self) -> Option<&RemoteConnectionName> {
        self.open.as_ref()?.original.as_ref()
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        let selected = self.selected_name()?;
        self.connections()
            .iter()
            .position(|entry| entry.name() == selected)
    }

    pub(crate) fn select(&mut self, index: usize) -> bool {
        if self.is_launching() {
            self.set_error("Wait for the Remote window to finish preparing or close to cancel");
            return false;
        }
        let Some(entry) = self.connections().get(index) else {
            return false;
        };
        if self.selected_name() == Some(entry.name()) {
            return false;
        }
        if self.open.as_ref().is_some_and(|open| open.dirty) {
            self.set_error("Save or close the editor before selecting another connection");
            return false;
        }
        self.load_connection(index);
        true
    }

    pub(crate) fn start_new(&mut self) -> bool {
        if self.is_launching() {
            self.set_error("Wait for the Remote window to finish preparing or close to cancel");
            return false;
        }
        if self.open.as_ref().is_some_and(|open| open.dirty) {
            self.set_error("Save or close the editor before creating another connection");
            return false;
        }
        self.load_new_draft();
        true
    }

    pub(crate) fn input(&self, field: RemoteConnectionManagerField) -> &TextInput {
        match field {
            RemoteConnectionManagerField::Name => &self.name,
            RemoteConnectionManagerField::Host => &self.host,
            RemoteConnectionManagerField::Workspace => &self.workspace,
        }
    }

    pub(crate) fn apply(&mut self, field: RemoteConnectionManagerField, command: TextInputCommand) {
        if self.is_launching() {
            self.set_error("Wait for the Remote window to finish preparing or close to cancel");
            return;
        }
        self.input_mut(field).apply(command);
        self.draft_changed();
    }

    pub(crate) fn apply_composition(
        &mut self,
        field: RemoteConnectionManagerField,
        event: TextInputCompositionEvent,
    ) {
        if self.is_launching() {
            return;
        }
        self.input_mut(field).apply_composition(event);
        self.draft_changed();
    }

    pub(crate) fn cancel_compositions_except(
        &mut self,
        active: Option<RemoteConnectionManagerField>,
    ) {
        for field in [
            RemoteConnectionManagerField::Name,
            RemoteConnectionManagerField::Host,
            RemoteConnectionManagerField::Workspace,
        ] {
            if Some(field) != active {
                self.input_mut(field).cancel_composition();
            }
        }
    }

    pub(crate) fn selected_text(&self, field: RemoteConnectionManagerField) -> Option<&str> {
        self.input(field).selected_text()
    }

    pub(crate) fn save_request(&mut self) -> Option<RemoteConnectionSaveRequest> {
        if self.is_launching() {
            self.set_error("Wait for the Remote window to finish preparing or close to cancel");
            return None;
        }
        let entry = match self.validated_entry() {
            Ok(entry) => entry,
            Err(error) => {
                self.set_error(error);
                return None;
            }
        };
        Some(match self.selected_name().cloned() {
            Some(original) => RemoteConnectionSaveRequest::Update { original, entry },
            None => RemoteConnectionSaveRequest::Create(entry),
        })
    }

    pub(crate) fn save_succeeded(&mut self, entry: RemoteConnectionEntry) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if let Some(original) = open.original.as_ref()
            && let Some(index) = open
                .connections
                .iter()
                .position(|existing| existing.name() == original)
        {
            open.connections.remove(index);
        }
        open.connections.push(entry.clone());
        open.connections
            .sort_by(|left, right| left.name().cmp(right.name()));
        open.original = Some(entry.name().clone());
        open.dirty = false;
        open.delete_confirmation = None;
        open.status = Some(ManagerStatus {
            message: format!("Saved `{}`", entry.name().as_str()),
            error: false,
        });
        self.replace_draft(&entry);
    }

    pub(crate) fn save_failed(&mut self, error: impl Into<String>) {
        self.set_error(error);
    }

    pub(crate) fn delete_request(&mut self) -> Option<RemoteConnectionName> {
        if self.is_launching() {
            self.set_error("Wait for the Remote window to finish preparing or close to cancel");
            return None;
        }
        let Some(name) = self.selected_name().cloned() else {
            self.set_error("Select a saved connection before deleting");
            return None;
        };
        let Some(open) = self.open.as_mut() else {
            return None;
        };
        if open.delete_confirmation.as_ref() == Some(&name) {
            return Some(name);
        }
        open.delete_confirmation = Some(name.clone());
        open.status = Some(ManagerStatus {
            message: format!("Press Confirm Delete to remove `{}`", name.as_str()),
            error: true,
        });
        None
    }

    pub(crate) fn delete_succeeded(&mut self, name: &RemoteConnectionName) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        open.connections.retain(|entry| entry.name() != name);
        open.dirty = false;
        open.delete_confirmation = None;
        open.status = Some(ManagerStatus {
            message: format!("Removed `{}`", name.as_str()),
            error: false,
        });
        if open.connections.is_empty() {
            self.load_new_draft();
            if let Some(open) = self.open.as_mut() {
                open.status = Some(ManagerStatus {
                    message: format!("Removed `{}`", name.as_str()),
                    error: false,
                });
            }
        } else {
            self.load_connection(0);
            if let Some(open) = self.open.as_mut() {
                open.status = Some(ManagerStatus {
                    message: format!("Removed `{}`", name.as_str()),
                    error: false,
                });
            }
        }
    }

    pub(crate) fn delete_label(&self) -> &'static str {
        if self.open.as_ref().is_some_and(|open| {
            open.delete_confirmation.as_ref() == open.original.as_ref() && open.original.is_some()
        }) {
            "Confirm Delete"
        } else {
            "Delete"
        }
    }

    pub(crate) fn status(&self) -> Option<(&str, bool)> {
        self.open
            .as_ref()?
            .status
            .as_ref()
            .map(|status| (status.message.as_str(), status.error))
    }

    pub(crate) const fn scroll_state(&self) -> ScrollState {
        self.scroll
    }

    pub(crate) fn apply_scroll(&mut self, command: ScrollCommand, metrics: ScrollMetrics) -> bool {
        self.scroll.apply(command, metrics, ScrollAxis::Vertical)
    }

    pub(crate) fn ensure_item_visible(&mut self, index: usize, metrics: ScrollMetrics) -> bool {
        self.apply_scroll(
            ScrollCommand::EnsureVisible(Rect::from_xywh(
                0.0,
                index as f32 * REMOTE_CONNECTION_MANAGER_ITEM_HEIGHT,
                metrics.content().width,
                REMOTE_CONNECTION_MANAGER_ITEM_HEIGHT,
            )),
            metrics,
        )
    }

    fn validated_entry(&self) -> Result<RemoteConnectionEntry, String> {
        let name =
            RemoteConnectionName::parse(self.name.text()).map_err(|error| error.to_string())?;
        let host = SshHost::parse(self.host.text()).map_err(|error| error.to_string())?;
        let workspace =
            RemoteWorkspacePath::parse(self.workspace.text()).map_err(|error| error.to_string())?;
        Ok(RemoteConnectionEntry::new(
            name,
            SshTarget::new(host, workspace),
        ))
    }

    fn load_connection(&mut self, index: usize) {
        let Some(entry) = self.connections().get(index).cloned() else {
            return;
        };
        self.replace_draft(&entry);
        if let Some(open) = self.open.as_mut() {
            open.original = Some(entry.name().clone());
            open.dirty = false;
            open.delete_confirmation = None;
            open.status = None;
        }
    }

    fn load_new_draft(&mut self) {
        replace_text(&mut self.name, "");
        replace_text(&mut self.host, "");
        replace_text(&mut self.workspace, "");
        if let Some(open) = self.open.as_mut() {
            open.original = None;
            open.dirty = false;
            open.delete_confirmation = None;
            open.status = None;
        }
    }

    fn replace_draft(&mut self, entry: &RemoteConnectionEntry) {
        replace_text(&mut self.name, entry.name().as_str());
        replace_text(&mut self.host, entry.target().host().as_str());
        replace_text(&mut self.workspace, entry.target().workspace().as_str());
    }

    fn input_mut(&mut self, field: RemoteConnectionManagerField) -> &mut TextInput {
        match field {
            RemoteConnectionManagerField::Name => &mut self.name,
            RemoteConnectionManagerField::Host => &mut self.host,
            RemoteConnectionManagerField::Workspace => &mut self.workspace,
        }
    }

    fn draft_changed(&mut self) {
        if let Some(open) = self.open.as_mut() {
            open.dirty = true;
            open.delete_confirmation = None;
            open.status = None;
        }
    }

    fn set_error(&mut self, error: impl Into<String>) {
        if let Some(open) = self.open.as_mut() {
            open.delete_confirmation = None;
            open.status = Some(ManagerStatus {
                message: error.into(),
                error: true,
            });
        }
    }
}

pub(crate) fn remote_connection_manager_item_id(index: usize) -> ElementId {
    ElementId::scoped(
        REMOTE_CONNECTION_MANAGER_SCOPE,
        FIRST_REMOTE_CONNECTION_MANAGER_ITEM.saturating_add(index as u32),
    )
}

pub(crate) fn remote_connection_manager_item_index(id: ElementId, count: usize) -> Option<usize> {
    (0..count).find(|index| remote_connection_manager_item_id(*index) == id)
}

fn replace_text(input: &mut TextInput, value: &str) {
    input.take_text();
    if !value.is_empty() {
        input.apply(TextInputCommand::Insert(value.into()));
    }
}

#[cfg(test)]
#[path = "remote_connection_manager_tests.rs"]
mod tests;
