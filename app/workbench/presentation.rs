use zeta_commands::AppCommandId;
use zeta_editor_host::{
    FILE_EDITOR_DOCUMENT, FILE_EDITOR_FIND_INPUT, FILE_EDITOR_REPLACE_INPUT, FileEditorHost,
    FileEditorPane, FileEditorPrompt, FileEditorSearchState,
};
use zeta_keybinding::{HostPlatform, KeySequence};
use zeta_settings::REMOTE_CONNECTION_SEARCH_INPUT;
use zeta_settings::REMOTE_TUNNEL_REMOTE_PORT;
use zeta_settings::RemoteConnectionManager;
use zeta_settings::RemoteConnectionManagerField;
use zeta_settings::RemoteConnectionManagerState;
use zeta_settings::RemoteConnectionPicker;
use zeta_settings::RemoteConnectionPickerState;
use zeta_settings::RemoteTunnelManager;
use zeta_settings::RemoteTunnelManagerState;
use zeta_terminal::{GridSize, ScreenBuffer, TerminalCore, TerminalMousePosition};
use zeta_ui_components::{
    InteractionRegion, Sash, SashOrientation, SashState, SashStyle, ScrollMetrics,
    ScrollbarPresentation,
};
use zui::ui::{
    Border, CaretVisibility, Color, CornerRadii, FontWeight, PaintRect, Rect, SceneCheckpoint,
    SplitViewOrientation, SplitViewResizeSnapshot, TextBlock, TextInputLayoutEngine, TextStyle,
    UiScene,
};

use crate::PaneBinding;
use crate::SessionSearchState;
use crate::{
    FIRST_TAB_CONTAINER_SESSION_TAB, MainSurfaceKind, SESSION_SEARCH_INPUT, TabContextMenu,
    TabContextMenuState, WINDOW, paint_chord_hint,
};
use crate::{
    InspectorPartState, PaneGroupId as PaneId, PaneInputKind, PaneMount, PanePart, PanePartSashes,
    PaneSplitId, TITLEBAR_HEIGHT, TabContainer, TabContainerPlacement, TabContainerState,
    TabContainerToolbar, TabGroupId, TabInput, TabInputKey, TabPart, Titlebar, TitlebarInsets,
    WorkbenchTab, WorkbenchTabGroup, mounted_tab_element_id, pane_group_element_id,
    tab_input_element_id, workbench_tab_groups,
};
use zeta_files::FilesLayout;
use zeta_files::FilesPane;
use zeta_files::FilesState;
use zeta_files::FilesToolbar;
use zeta_files::{
    DIRECTORY_SEARCH_INPUT, DirectoryPicker, DirectoryPickerState, FILE_SEARCH_INPUT,
};
use zeta_scm::EditorPane;
use zeta_scm::ScmState;
use zeta_scm::{GIT_BRANCH_SEARCH_INPUT, GitBranchContextMenu, GitBranchContextMenuState};
use zeta_session::SessionPaneContext;
use zeta_session::SessionPaneLayout;
use zeta_session::SessionPaneState;
use zeta_session::SessionPaneView;
use zeta_session::draw_session_pane;
use zeta_terminal_runtime::TerminalSelectionRange;
use zeta_ui_theme::UiTheme;

type PaneViewMount<'a> = PaneMount<'a, PaneBinding>;
use crate::InspectorLayoutSpec;
use crate::PaneGroupLayout;
use crate::PartVisibility;
use crate::WorkbenchLayout;
use crate::WorkbenchLayoutSpec;
use zeta_editor::CodeEditorStyle;
use zeta_settings::AppearanceSettingsSnapshot;
use zeta_settings::GeneralSettingsSnapshot;
use zeta_settings::KeybindingSettingsSnapshot;
use zeta_settings::RemoteSettingsSnapshot;
use zeta_settings::SettingsFeatureSnapshot;
use zeta_settings::SettingsPaneStyle;
use zeta_settings::SettingsPaneView;
use zeta_settings::SettingsState;
use zui::ui::{
    AccessibilityRole, ComponentContext, CursorFeedback, ElementId, InteractionFrame,
    InteractionFrameCheckpoint, UiDispatch, UiFrame,
};
use zui::window::WindowControlInsets;

const COMPOSER_HEIGHT: f32 = 44.0;

pub const MAIN_SURFACE: ElementId = ElementId::scoped(1, 3);
pub const TERMINAL_OUTPUT: ElementId = ElementId::scoped(1, 4);
pub const TAB_CONTAINER_RESIZE_HANDLE: ElementId = ElementId::scoped(1, 16);
pub const INSPECTOR_RESIZE_HANDLE: ElementId = ElementId::scoped(1, 51);

pub(crate) use crate::LogicalViewport;

pub trait WorkbenchKeybindings {
    fn pending_keybinding(&self) -> Option<(&KeySequence, usize)>;
    fn platform(&self) -> HostPlatform;
    fn binding_for_command(&self, command: AppCommandId) -> Option<&KeySequence>;
}

impl<C> WorkbenchKeybindings for zeta_keybindings_host::Keybindings<C>
where
    C: zeta_keybindings_host::KeybindingCatalog<Command = AppCommandId>,
{
    fn pending_keybinding(&self) -> Option<(&KeySequence, usize)> {
        self.pending_keybinding()
    }

    fn platform(&self) -> HostPlatform {
        self.platform()
    }

    fn binding_for_command(&self, command: AppCommandId) -> Option<&KeySequence> {
        self.binding_for_command(command)
    }
}

#[derive(Clone)]
pub struct EnvironmentContextView<'a> {
    pub location: &'a str,
    pub working_directory: &'a str,
    pub git_branch: &'a str,
    pub diff_summary: String,
    pub upstream_distance: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkbenchSceneLayout {
    pub workbench: WorkbenchLayout,
    pub output: Rect,
    pub session_header: Rect,
    pub thread_timeline: Rect,
    pub session_pane_layout: SessionPaneLayout,
    pub composer_panel: Rect,
    pub composer_info_bar: Rect,
    pub composer_toolbar: Rect,
    pub composer: Rect,
}

impl WorkbenchSceneLayout {
    pub fn titlebar(self) -> Rect {
        self.workbench.titlebar()
    }

    pub fn tab_container(self) -> Option<Rect> {
        self.workbench.tab_container()
    }

    fn tab_container_sash_track(self) -> Option<Rect> {
        self.workbench.tab_container_sash_track()
    }

    fn inspector(self) -> Option<Rect> {
        self.workbench.inspector()
    }

    fn inspector_sash_track(self) -> Option<Rect> {
        self.workbench.inspector_sash_track()
    }

    fn inspector_resize_snapshot(self) -> Option<SplitViewResizeSnapshot> {
        self.workbench.inspector_resize_snapshot()
    }

    pub fn main(self) -> Rect {
        self.workbench.main()
    }

    pub fn for_viewport(
        viewport: LogicalViewport,
        tab_container: TabContainerState,
        inspector_part: InspectorPartState,
    ) -> Option<Self> {
        Self::for_viewport_with_composer_and_interaction_height(
            viewport,
            tab_container,
            inspector_part,
            COMPOSER_HEIGHT,
            0.0,
        )
    }

    #[cfg(test)]
    pub fn for_viewport_with_composer_height(
        viewport: LogicalViewport,
        tab_container: TabContainerState,
        inspector_part: InspectorPartState,
        preferred_composer_height: f32,
    ) -> Option<Self> {
        Self::for_viewport_with_composer_and_interaction_height(
            viewport,
            tab_container,
            inspector_part,
            preferred_composer_height,
            0.0,
        )
    }

    fn for_viewport_with_composer_and_interaction_height(
        viewport: LogicalViewport,
        tab_container: TabContainerState,
        inspector_part: InspectorPartState,
        preferred_composer_height: f32,
        preferred_interaction_height: f32,
    ) -> Option<Self> {
        let workbench = WorkbenchLayoutSpec::new(
            TITLEBAR_HEIGHT,
            tab_container.layout_spec(),
            InspectorLayoutSpec::new(
                if inspector_part.is_expanded() {
                    PartVisibility::Expanded
                } else {
                    PartVisibility::Collapsed
                },
                inspector_part.preferred_width(),
                360.0,
                800.0,
                400.0,
            ),
        )
        .for_viewport(viewport)?;
        let main = workbench.main();
        let session_pane_layout = SessionPaneLayout::for_bounds(
            main,
            preferred_composer_height.max(COMPOSER_HEIGHT),
            preferred_interaction_height,
        );
        let composer_panel = session_pane_layout.composer();
        let output = composer_panel.output();
        Some(Self {
            workbench,
            output,
            session_header: session_pane_layout.header(),
            thread_timeline: session_pane_layout.timeline(),
            session_pane_layout,
            composer_panel: composer_panel.panel(),
            composer_info_bar: composer_panel.info_bar(),
            composer_toolbar: composer_panel.toolbar(),
            composer: composer_panel.editor(),
        })
    }
}

pub fn inspector_resize_snapshot_for_viewport(
    viewport: LogicalViewport,
    tab_container: TabContainerState,
    inspector_part: InspectorPartState,
) -> Option<SplitViewResizeSnapshot> {
    WorkbenchSceneLayout::for_viewport(viewport, tab_container, inspector_part)
        .and_then(WorkbenchSceneLayout::inspector_resize_snapshot)
}

pub struct WorkbenchPresentation {
    pub frame: UiFrame<InteractionFrame>,
    pub ime_cursor_area: Option<Rect>,
    pub path_picker_scroll_metrics: Option<ScrollMetrics>,
    pub path_picker_item_viewport: Option<Rect>,
    pub remote_connection_picker_scroll_metrics: Option<ScrollMetrics>,
    pub remote_connection_picker_item_viewport: Option<Rect>,
    pub remote_connection_manager_scroll_metrics: Option<ScrollMetrics>,
    pub remote_connection_manager_list_viewport: Option<Rect>,
    pub remote_tunnel_manager_scroll_metrics: Option<ScrollMetrics>,
    pub remote_tunnel_manager_list_viewport: Option<Rect>,
    base_checkpoint: Option<WorkbenchBaseCheckpoint>,
}

impl WorkbenchPresentation {
    pub fn frame(&self) -> &UiFrame<InteractionFrame> {
        &self.frame
    }

    pub fn scene_mut(&mut self) -> &mut UiScene {
        self.frame.scene_mut()
    }

    pub fn interaction_frame(&self) -> &InteractionFrame {
        self.frame.interaction()
    }

    pub fn interaction_frame_mut(&mut self) -> &mut InteractionFrame {
        self.frame.interaction_mut()
    }

    pub fn element_bounds(&self, id: ElementId) -> Option<Rect> {
        self.interaction_frame().node(id).map(|node| node.bounds())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct WorkbenchBaseCheckpoint {
    scene: SceneCheckpoint,
    interaction: InteractionFrameCheckpoint,
    ime_cursor_area: Option<Rect>,
    remote_connection_manager_scroll_metrics: Option<ScrollMetrics>,
    remote_connection_manager_list_viewport: Option<Rect>,
}

struct WorkbenchOverlayPresentation {
    ime_cursor_area: Option<Rect>,
    path_picker_scroll_metrics: Option<ScrollMetrics>,
    path_picker_item_viewport: Option<Rect>,
    remote_connection_picker_scroll_metrics: Option<ScrollMetrics>,
    remote_connection_picker_item_viewport: Option<Rect>,
    remote_connection_manager_scroll_metrics: Option<ScrollMetrics>,
    remote_connection_manager_list_viewport: Option<Rect>,
    remote_tunnel_manager_scroll_metrics: Option<ScrollMetrics>,
    remote_tunnel_manager_list_viewport: Option<Rect>,
}

#[derive(Clone, Copy)]
pub struct PaneView<'a> {
    pub pane_id: Option<PaneId>,
    pub kind: PaneInputKind,
    pub core: Option<&'a TerminalCore>,
    pub scroll_offset: usize,
    pub scrollbar_presentation: ScrollbarPresentation,
    pub selection: Option<TerminalSelectionRange>,
}

#[derive(Clone)]
pub struct WorkbenchPresentationModel<'a> {
    pub product_name: &'a str,
    pub palette: UiTheme,
    pub terminal: Option<&'a TerminalCore>,
    pub terminal_panes: &'a [PaneView<'a>],
    pub pane_group: Option<&'a PanePart>,
    pub active_pane: Option<PaneViewMount<'a>>,
    pub terminal_pane_resize_split: Option<PaneSplitId>,
    pub terminal_scroll_offset: usize,
    pub terminal_scrollbar_presentation: ScrollbarPresentation,
    pub terminal_selection: Option<TerminalSelectionRange>,
    pub main_surface: MainSurfaceKind,
    pub file_editor_host: &'a FileEditorHost,
    pub file_editor_prompt: FileEditorPrompt,
    pub file_editor_search: &'a FileEditorSearchState,
    pub file_editor_diagnostics: &'a [zeta_editor::CodeEditorDiagnostic],
    pub language_hover: Option<&'a zeta_lsp_manager::LanguageHover>,
    pub language_completions: Option<&'a zeta_lsp_manager::LanguageCompletions>,
    pub completion_selection: usize,
    pub code_editor_style: &'a CodeEditorStyle,
    pub session_pane: &'a SessionPaneState,
    pub environment_context: EnvironmentContextView<'a>,
    pub session_search: &'a SessionSearchState,
    pub tab_part: &'a TabPart,
    pub active_tab_input: Option<&'a TabInputKey>,
    pub caret_visibility: CaretVisibility,
    pub dispatch: &'a UiDispatch,
    pub tab_container: TabContainerState,
    pub inspector_part: InspectorPartState,
    pub files: &'a FilesState,
    pub scm: &'a ScmState,
    pub tab_context_menu: TabContextMenuState,
    pub git_branch_context_menu: &'a GitBranchContextMenuState,
    pub path_picker: &'a DirectoryPickerState,
    pub remote_connection_picker: &'a RemoteConnectionPickerState,
    pub remote_connection_manager: &'a RemoteConnectionManagerState,
    pub remote_tunnel_manager: &'a RemoteTunnelManagerState,
    pub keybindings: &'a dyn WorkbenchKeybindings,
    pub settings: &'a SettingsState,
    pub keybinding_diagnostics: &'a [String],
    pub theme_scheme: zeta_theme::ColorScheme,
    pub theme_follows_system: bool,
    pub window_control_insets: WindowControlInsets,
    pub pointer_position: Option<zui::ui::Point>,
}

#[derive(Clone)]
struct TabContainerView<'a> {
    title: &'a str,
    context: EnvironmentContextView<'a>,
    search: &'a SessionSearchState,
    tab_part: &'a TabPart,
    selected_id: ElementId,
    visible_action_bar_tab: Option<&'a TabInputKey>,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

#[derive(Clone, Copy)]
struct FileEditorPresentationView<'a> {
    host: &'a FileEditorHost,
    prompt: FileEditorPrompt,
    search: &'a FileEditorSearchState,
    diagnostics: &'a [zeta_editor::CodeEditorDiagnostic],
    language_hover: Option<&'a zeta_lsp_manager::LanguageHover>,
    language_completions: Option<&'a zeta_lsp_manager::LanguageCompletions>,
    completion_selection: usize,
    style: &'a CodeEditorStyle,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
    pointer_position: Option<zui::ui::Point>,
}

#[derive(Clone)]
struct MainPresentationView<'a> {
    terminal: PaneView<'a>,
    terminal_panes: &'a [PaneView<'a>],
    pane_group: Option<&'a PanePart>,
    active_pane: Option<PaneViewMount<'a>>,
    terminal_pane_resize_split: Option<PaneSplitId>,
    main_surface: MainSurfaceKind,
    active_tab_input: Option<&'a TabInputKey>,
    settings: &'a SettingsState,
    remote_connection_manager: &'a RemoteConnectionManagerState,
    session_title: &'a str,
    session_pane: &'a SessionPaneState,
    session_pane_context: &'a SessionPaneContext,
    files: &'a FilesState,
    scm: &'a ScmState,
    environment_context: EnvironmentContextView<'a>,
    keybindings: &'a dyn WorkbenchKeybindings,
    keybinding_diagnostics: &'a [String],
    theme_scheme: zeta_theme::ColorScheme,
    theme_follows_system: bool,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MainDrawResult {
    ime_cursor_area: Option<Rect>,
    remote_connection_manager_scroll_metrics: Option<ScrollMetrics>,
    remote_connection_manager_list_viewport: Option<Rect>,
}

#[cfg(test)]
pub fn build_workbench_presentation(
    viewport: LogicalViewport,
    model: WorkbenchPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> WorkbenchPresentation {
    build_workbench_presentation_with_bindings(
        viewport,
        model,
        text_layout,
        SashState::Resting,
        None,
    )
}

pub fn build_workbench_presentation_with_animation_bindings(
    viewport: LogicalViewport,
    model: WorkbenchPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    inspector_sash_state: SashState,
    animation_bindings: &mut dyn zui::ui::AnimationBinding,
) -> WorkbenchPresentation {
    build_workbench_presentation_with_bindings(
        viewport,
        model,
        text_layout,
        inspector_sash_state,
        Some(animation_bindings),
    )
}

fn build_workbench_presentation_with_bindings(
    viewport: LogicalViewport,
    model: WorkbenchPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    inspector_sash_state: SashState,
    mut animation_bindings: Option<&mut dyn zui::ui::AnimationBinding>,
) -> WorkbenchPresentation {
    let palette = model.palette;
    let mut frame = UiFrame::<InteractionFrame>::new(palette.workbench_background);
    frame.draw_component(&InteractionRegion::new(
        "Window",
        WINDOW,
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        AccessibilityRole::Window,
        model.product_name,
    ));
    let Some(layout) = WorkbenchSceneLayout::for_viewport_with_composer_and_interaction_height(
        viewport,
        model.tab_container,
        model.inspector_part,
        model.session_pane.composer_preferred_height(),
        model
            .session_pane
            .composer_interaction_view()
            .map_or(0.0, |view| {
                zeta_session::interaction_preferred_height(view.items().len())
            }),
    ) else {
        draw_compact_scene(frame.scene_mut(), viewport, model.product_name, palette);
        return WorkbenchPresentation {
            frame,
            ime_cursor_area: None,
            path_picker_scroll_metrics: None,
            path_picker_item_viewport: None,
            remote_connection_picker_scroll_metrics: None,
            remote_connection_picker_item_viewport: None,
            remote_connection_manager_scroll_metrics: None,
            remote_connection_manager_list_viewport: None,
            remote_tunnel_manager_scroll_metrics: None,
            remote_tunnel_manager_list_viewport: None,
            base_checkpoint: None,
        };
    };

    let title = if model.main_surface == MainSurfaceKind::Terminal {
        model
            .terminal
            .and_then(TerminalCore::title)
            .unwrap_or("Terminal")
    } else {
        model
            .session_pane
            .thread()
            .map(|thread| thread.title.as_str())
            .unwrap_or(model.product_name)
    };
    let session_title = model
        .active_tab_input
        .and_then(|selected| model.tab_part.input(selected))
        .map(TabInput::title)
        .unwrap_or("New session");
    let session_pane_context = SessionPaneContext::new(
        model.environment_context.location,
        model.environment_context.working_directory,
        model.environment_context.git_branch,
        &model.environment_context.diff_summary,
    );
    let selected_id = tab_input_element_id(
        model.tab_part,
        model.active_tab_input,
        TabContainerPlacement::Body,
    );
    let mut titlebar = Titlebar::new(
        layout.titlebar(),
        crate::WorkbenchUiStyle::from_theme(palette),
        model.tab_part,
        model.active_tab_input,
        model.tab_container.is_expanded(),
        model.active_pane.map(|pane| pane.kind()),
        TitlebarInsets::new(
            model.window_control_insets.left(),
            model.window_control_insets.right(),
        ),
        model.dispatch,
    );
    if let Some(tab) = model.tab_context_menu.target_tab().and_then(|tab| {
        mounted_tab_element_id(model.tab_part, tab, TabContainerPlacement::Titlebar)
    }) {
        titlebar = titlebar.with_visible_tab_action_bar(tab);
    }
    frame.draw_component(&titlebar);
    let session_search_caret = if let Some(bounds) = layout.tab_container() {
        frame.with_context(|context| {
            draw_tab_container(
                context,
                bounds,
                Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
                TabContainerView {
                    title,
                    context: model.environment_context.clone(),
                    search: model.session_search,
                    tab_part: model.tab_part,
                    selected_id,
                    visible_action_bar_tab: model.tab_context_menu.target_tab(),
                    caret_visibility: model.caret_visibility,
                    dispatch: model.dispatch,
                },
                text_layout,
                palette,
            )
        })
    } else {
        None
    };
    let file_editor_caret = if let Some(bounds) = layout.inspector() {
        if model.main_surface == MainSurfaceKind::Editor {
            frame.with_context(|context| {
                draw_file_editor_inspector(
                    context,
                    bounds,
                    FileEditorPresentationView {
                        host: model.file_editor_host,
                        prompt: model.file_editor_prompt,
                        search: model.file_editor_search,
                        diagnostics: model.file_editor_diagnostics,
                        language_hover: model.language_hover,
                        language_completions: model.language_completions,
                        completion_selection: model.completion_selection,
                        style: model.code_editor_style,
                        caret_visibility: model.caret_visibility,
                        dispatch: model.dispatch,
                        pointer_position: model.pointer_position,
                    },
                    text_layout,
                    palette,
                )
            })
        } else {
            None
        }
    } else {
        None
    };
    let main_draw = |context: &mut ComponentContext<'_, '_>| {
        draw_main(
            context,
            layout,
            MainPresentationView {
                terminal: PaneView {
                    pane_id: None,
                    kind: PaneInputKind::Terminal,
                    core: model.terminal,
                    scroll_offset: model.terminal_scroll_offset,
                    scrollbar_presentation: model.terminal_scrollbar_presentation,
                    selection: model.terminal_selection,
                },
                terminal_panes: model.terminal_panes,
                pane_group: model.pane_group,
                active_pane: model.active_pane,
                terminal_pane_resize_split: model.terminal_pane_resize_split,
                main_surface: model.main_surface,
                active_tab_input: model.active_tab_input,
                settings: model.settings,
                remote_connection_manager: model.remote_connection_manager,
                session_title,
                session_pane: model.session_pane,
                session_pane_context: &session_pane_context,
                files: model.files,
                scm: model.scm,
                environment_context: model.environment_context.clone(),
                keybindings: model.keybindings,
                keybinding_diagnostics: model.keybinding_diagnostics,
                theme_scheme: model.theme_scheme,
                theme_follows_system: model.theme_follows_system,
                caret_visibility: model.caret_visibility,
                dispatch: model.dispatch,
            },
            palette,
            text_layout,
        )
    };
    let main_draw = match animation_bindings.as_deref_mut() {
        Some(animation_bindings) => frame.with_animation_bindings(animation_bindings, main_draw),
        None => frame.with_context(main_draw),
    };
    if let Some(bounds) = layout.inspector() {
        draw_inspector_border(frame.scene_mut(), bounds, palette);
    }
    if let Some(bounds) = layout.inspector_sash_track() {
        frame.with_context(|context| {
            draw_sash(
                context,
                bounds,
                SashOrientation::Vertical,
                inspector_sash_state,
                INSPECTOR_RESIZE_HANDLE,
                "InspectorPartResizeHandle",
                "Resize inspector",
                palette,
            )
        });
    }
    let ime_cursor_area = if model.dispatch.is_focused(FILE_EDITOR_DOCUMENT)
        || model.dispatch.is_focused(FILE_EDITOR_FIND_INPUT)
        || model.dispatch.is_focused(FILE_EDITOR_REPLACE_INPUT)
    {
        file_editor_caret
    } else if model.dispatch.is_focused(SESSION_SEARCH_INPUT) {
        session_search_caret
    } else {
        main_draw.ime_cursor_area
    };
    if let Some(bounds) = layout.tab_container_sash_track() {
        frame.with_context(|context| {
            draw_sash(
                context,
                bounds,
                SashOrientation::Vertical,
                model.tab_container.sash_state(),
                TAB_CONTAINER_RESIZE_HANDLE,
                "TabContainerResizeHandle",
                "Resize tabs",
                palette,
            )
        });
    }
    let base_checkpoint = WorkbenchBaseCheckpoint {
        scene: frame.scene().checkpoint(),
        interaction: frame.interaction().checkpoint(),
        ime_cursor_area,
        remote_connection_manager_scroll_metrics: main_draw
            .remote_connection_manager_scroll_metrics,
        remote_connection_manager_list_viewport: main_draw.remote_connection_manager_list_viewport,
    };
    let overlay =
        draw_workbench_overlays(&mut frame, viewport, &model, text_layout, ime_cursor_area);
    WorkbenchPresentation {
        frame,
        ime_cursor_area: overlay.ime_cursor_area,
        path_picker_scroll_metrics: overlay.path_picker_scroll_metrics,
        path_picker_item_viewport: overlay.path_picker_item_viewport,
        remote_connection_picker_scroll_metrics: overlay.remote_connection_picker_scroll_metrics,
        remote_connection_picker_item_viewport: overlay.remote_connection_picker_item_viewport,
        remote_connection_manager_scroll_metrics: overlay
            .remote_connection_manager_scroll_metrics
            .or(main_draw.remote_connection_manager_scroll_metrics),
        remote_connection_manager_list_viewport: overlay
            .remote_connection_manager_list_viewport
            .or(main_draw.remote_connection_manager_list_viewport),
        remote_tunnel_manager_scroll_metrics: overlay.remote_tunnel_manager_scroll_metrics,
        remote_tunnel_manager_list_viewport: overlay.remote_tunnel_manager_list_viewport,
        base_checkpoint: Some(base_checkpoint),
    }
}

/// Replaces only volatile Workbench overlays while retaining base layout, paint, and interaction data.
pub fn rebuild_workbench_overlays(
    presentation: &mut WorkbenchPresentation,
    viewport: LogicalViewport,
    model: WorkbenchPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> bool {
    let Some(base) = presentation.base_checkpoint.clone() else {
        return false;
    };
    presentation.scene_mut().restore(&base.scene);
    presentation
        .interaction_frame_mut()
        .restore(base.interaction);
    let overlay = draw_workbench_overlays(
        &mut presentation.frame,
        viewport,
        &model,
        text_layout,
        base.ime_cursor_area,
    );
    presentation.ime_cursor_area = overlay.ime_cursor_area;
    presentation.path_picker_scroll_metrics = overlay.path_picker_scroll_metrics;
    presentation.path_picker_item_viewport = overlay.path_picker_item_viewport;
    presentation.remote_connection_picker_scroll_metrics =
        overlay.remote_connection_picker_scroll_metrics;
    presentation.remote_connection_picker_item_viewport =
        overlay.remote_connection_picker_item_viewport;
    presentation.remote_connection_manager_scroll_metrics = overlay
        .remote_connection_manager_scroll_metrics
        .or(base.remote_connection_manager_scroll_metrics);
    presentation.remote_connection_manager_list_viewport = overlay
        .remote_connection_manager_list_viewport
        .or(base.remote_connection_manager_list_viewport);
    presentation.remote_tunnel_manager_scroll_metrics =
        overlay.remote_tunnel_manager_scroll_metrics;
    presentation.remote_tunnel_manager_list_viewport = overlay.remote_tunnel_manager_list_viewport;
    true
}

fn draw_workbench_overlays(
    frame: &mut UiFrame<InteractionFrame>,
    viewport: LogicalViewport,
    model: &WorkbenchPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    mut ime_cursor_area: Option<Rect>,
) -> WorkbenchOverlayPresentation {
    let palette = model.palette;
    let mut path_picker_scroll_metrics = None;
    let mut path_picker_item_viewport = None;
    let mut remote_connection_picker_scroll_metrics = None;
    let mut remote_connection_picker_item_viewport = None;
    let mut remote_connection_manager_scroll_metrics = None;
    let mut remote_connection_manager_list_viewport = None;
    let mut remote_tunnel_manager_scroll_metrics = None;
    let mut remote_tunnel_manager_list_viewport = None;
    if let Some(context_menu) = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.tab_part,
        &model.tab_context_menu,
        model.caret_visibility,
        crate::TabContextMenuStyle::from_theme(palette),
        WINDOW,
        text_layout,
        model.dispatch,
    ) {
        frame.draw_component(&context_menu);
    }
    if let Some(path_picker) = DirectoryPicker::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.path_picker,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        path_picker_scroll_metrics = path_picker.scroll_metrics();
        path_picker_item_viewport = Some(path_picker.item_viewport_bounds());
        if model.dispatch.is_focused(DIRECTORY_SEARCH_INPUT) {
            ime_cursor_area = path_picker.search_caret_bounds();
        }
        frame.draw_component(&path_picker);
    }
    if let Some(connection_picker) = RemoteConnectionPicker::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.remote_connection_picker,
        model.caret_visibility,
        zeta_settings::RemoteUiStyle::from_theme(palette),
        text_layout,
        model.dispatch,
        WINDOW,
    ) {
        remote_connection_picker_scroll_metrics = connection_picker.scroll_metrics();
        remote_connection_picker_item_viewport = Some(connection_picker.item_viewport_bounds());
        if model.dispatch.is_focused(REMOTE_CONNECTION_SEARCH_INPUT) {
            ime_cursor_area = connection_picker.search_caret_bounds();
        }
        frame.draw_component(&connection_picker);
    }
    if let Some(connection_manager) = RemoteConnectionManager::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.remote_connection_manager,
        model.caret_visibility,
        zeta_settings::RemoteUiStyle::from_theme(palette),
        text_layout,
        model.dispatch,
        WINDOW,
    ) {
        remote_connection_manager_scroll_metrics = Some(connection_manager.list_scroll_metrics());
        remote_connection_manager_list_viewport = Some(connection_manager.list_viewport_bounds());
        for field in [
            RemoteConnectionManagerField::Name,
            RemoteConnectionManagerField::Host,
            RemoteConnectionManagerField::Directory,
        ] {
            if model.dispatch.is_focused(field.element_id()) {
                ime_cursor_area = connection_manager.caret_bounds(field);
            }
        }
        frame.draw_component(&connection_manager);
    }
    if let Some(tunnel_manager) = RemoteTunnelManager::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.remote_tunnel_manager,
        model.caret_visibility,
        zeta_settings::RemoteUiStyle::from_theme(palette),
        text_layout,
        model.dispatch,
        WINDOW,
    ) {
        remote_tunnel_manager_scroll_metrics = Some(tunnel_manager.list_scroll_metrics());
        remote_tunnel_manager_list_viewport = Some(tunnel_manager.list_viewport_bounds());
        if model.dispatch.is_focused(REMOTE_TUNNEL_REMOTE_PORT) {
            ime_cursor_area = tunnel_manager.remote_port_caret_bounds();
        }
        frame.draw_component(&tunnel_manager);
    }
    if let Some(branch_menu) = GitBranchContextMenu::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.git_branch_context_menu,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        if model.dispatch.is_focused(GIT_BRANCH_SEARCH_INPUT) {
            ime_cursor_area = branch_menu.search_caret_bounds();
        }
        frame.draw_component(&branch_menu);
    }
    let viewport_bounds = Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height);
    if let Some((keybinding, entered)) = model.keybindings.pending_keybinding() {
        paint_chord_hint(
            frame.scene_mut(),
            viewport_bounds,
            keybinding,
            entered,
            model.keybindings.platform(),
        );
    }
    let shortcut_rows = zeta_settings::keyboard_shortcut_rows(|command| {
        model.keybindings.binding_for_command(command)
    });
    zeta_settings::draw_keyboard_shortcuts_overlay(
        frame,
        viewport_bounds,
        model.settings.keyboard_shortcuts(),
        &shortcut_rows,
        model.keybinding_diagnostics,
        WINDOW,
        model.keybindings.platform(),
        model.dispatch,
    );
    WorkbenchOverlayPresentation {
        ime_cursor_area,
        path_picker_scroll_metrics,
        path_picker_item_viewport,
        remote_connection_picker_scroll_metrics,
        remote_connection_picker_item_viewport,
        remote_connection_manager_scroll_metrics,
        remote_connection_manager_list_viewport,
        remote_tunnel_manager_scroll_metrics,
        remote_tunnel_manager_list_viewport,
    }
}

pub fn terminal_grid_size_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    tab_container: TabContainerState,
    inspector_part: InspectorPartState,
) -> GridSize {
    let Some(layout) = WorkbenchSceneLayout::for_viewport(viewport, tab_container, inspector_part)
    else {
        return GridSize::default();
    };
    let bounds = terminal_content_bounds(layout, active_screen);
    zeta_terminal_runtime::grid_size(bounds)
}

pub fn terminal_grid_size_for_bounds(bounds: Rect) -> GridSize {
    zeta_terminal_runtime::grid_size(bounds)
}

pub fn terminal_pane_bounds_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    tab_container: TabContainerState,
    inspector_part: InspectorPartState,
    group: &PanePart,
) -> Vec<(PaneId, Rect)> {
    let Some(layout) = WorkbenchSceneLayout::for_viewport(viewport, tab_container, inspector_part)
    else {
        return Vec::new();
    };
    let bounds = terminal_content_bounds(layout, active_screen);
    PaneGroupLayout::for_tree(bounds, group.tree())
        .leaves()
        .iter()
        .map(|leaf| (leaf.id(), leaf.bounds()))
        .collect()
}

pub fn terminal_mouse_position_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    tab_container: TabContainerState,
    inspector_part: InspectorPartState,
    point: zui::ui::Point,
) -> Option<TerminalMousePosition> {
    let layout = WorkbenchSceneLayout::for_viewport(viewport, tab_container, inspector_part)?;
    let bounds = terminal_content_bounds(layout, active_screen);
    if !bounds.contains(point) {
        return None;
    }
    zeta_terminal_runtime::mouse_position(bounds, point)
}

pub fn terminal_pane_mouse_position_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    tab_container: TabContainerState,
    inspector_part: InspectorPartState,
    group: &PanePart,
    point: zui::ui::Point,
) -> Option<(PaneId, TerminalMousePosition)> {
    let Some(layout) = WorkbenchSceneLayout::for_viewport(viewport, tab_container, inspector_part)
    else {
        return None;
    };
    let content_bounds = terminal_content_bounds(layout, active_screen);
    let pane_geometry = PaneGroupLayout::for_tree(content_bounds, group.tree());
    let leaf = pane_geometry
        .leaves()
        .iter()
        .find(|leaf| leaf.bounds().contains(point))?;
    let bounds = leaf.bounds();
    zeta_terminal_runtime::mouse_position(bounds, point).map(|position| (leaf.id(), position))
}

fn draw_files_pane(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    files: &FilesState,
    environment_context: &EnvironmentContextView<'_>,
    parent: ElementId,
    caret_visibility: CaretVisibility,
    dispatch: &UiDispatch,
    text_layout: &mut TextInputLayoutEngine,
    palette: UiTheme,
) -> Option<Rect> {
    let layout = FilesLayout::for_bounds(bounds);
    let files_toolbar = FilesToolbar::new(
        layout.toolbar(),
        files,
        environment_context.upstream_distance,
        caret_visibility,
        zeta_files::FilesToolbarStyle::from_theme(palette),
        parent,
        text_layout,
        dispatch,
    );
    let search_caret = files_toolbar.search_caret_bounds();
    context.draw_component(&files_toolbar);
    let files_style = zeta_files::FilesPaneStyle::from_theme(palette);
    context.draw_component(&FilesPane::new(
        layout.content(),
        files,
        parent,
        &files_style,
        dispatch,
    ));
    dispatch
        .is_focused(FILE_SEARCH_INPUT)
        .then_some(search_caret)
        .flatten()
}

fn draw_changes_pane(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    scm: &ScmState,
    parent: ElementId,
    palette: UiTheme,
) {
    context.draw_component(&EditorPane::new(
        bounds,
        scm.editor(),
        zeta_scm::ScmPaneStyle::from_theme(palette),
        parent,
    ));
}

fn draw_file_editor_inspector(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    view: FileEditorPresentationView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: UiTheme,
) -> Option<Rect> {
    draw_main_surface(context.scene_mut(), bounds, palette);
    let pane = FileEditorPane::new(
        bounds,
        view.host,
        view.style.clone(),
        palette,
        if view.dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
            view.caret_visibility
        } else {
            CaretVisibility::Hidden
        },
    )
    .with_parent(WINDOW)
    .with_prompt(view.prompt)
    .with_diagnostics(view.diagnostics)
    .with_language_features(view.language_hover, view.language_completions)
    .with_completion_selection(view.completion_selection)
    .with_pointer_position(view.pointer_position)
    .with_search(
        view.search,
        text_layout,
        view.dispatch,
        view.caret_visibility,
    );
    let ime_cursor_area = if view.dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
        pane.caret_bounds()
    } else {
        view.dispatch
            .focused()
            .and_then(|focused| pane.search_caret_bounds(focused))
    };
    context.draw_component(&pane);
    ime_cursor_area
}

/// Paints a Desktop-owned Workbench surface before feature content.
fn draw_main_surface(scene: &mut UiScene, bounds: Rect, palette: UiTheme) {
    scene.draw_rect(PaintRect::new(bounds, palette.side_bar_background));
}

/// Paints the Desktop-owned outer border of the Inspector Part after feature content.
///
/// This boundary separates the right Inspector slot from the main pane.
/// Files and SCM components own only their internal geometry and
/// must not redraw this edge.
pub fn draw_inspector_border(scene: &mut UiScene, bounds: Rect, palette: UiTheme) {
    scene.draw_rect(
        PaintRect::new(bounds, Color::TRANSPARENT).with_border(Border::new(
            zui::ui::Edges::new(0.0, 0.0, 0.0, 1.0),
            palette.border,
        )),
    );
}

fn draw_tab_container(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    viewport: Rect,
    view: TabContainerView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: UiTheme,
) -> Option<Rect> {
    context.scene_mut().draw_rect(
        PaintRect::new(bounds, palette.side_bar_background).with_border(Border::new(
            zui::ui::Edges::new(0.0, 1.0, 0.0, 0.0),
            palette.border,
        )),
    );
    let toolbar = TabContainerToolbar::new(
        bounds,
        view.search.input(),
        view.caret_visibility,
        crate::WorkbenchUiStyle::from_theme(palette),
        text_layout,
        view.dispatch,
    );
    let search_caret = toolbar.search_caret_bounds();
    context.draw_component(&toolbar);
    let has_session_input = view.tab_part.inputs().any(TabInput::is_session);
    let mut groups = workbench_tab_groups(view.tab_part, TabContainerPlacement::Body, |input| {
        input.is_settings() || view.search.matches_session_name(input.title())
    });
    if !has_session_input && view.search.matches_session_name(view.title) {
        let fallback = WorkbenchTab::new(
            FIRST_TAB_CONTAINER_SESSION_TAB,
            crate::FIRST_TAB_CONTAINER_SESSION_ACTION,
            crate::FIRST_TAB_CONTAINER_SESSION_CLOSE,
            view.title,
            view.context.working_directory,
            crate::TabStatus::idle("Ready"),
            false,
        );
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.id() == TabGroupId::DEFAULT)
        {
            group.insert_tab(0, fallback);
        } else {
            groups.push(WorkbenchTabGroup::new(
                TabGroupId::DEFAULT,
                None,
                false,
                vec![fallback],
            ));
        }
    }
    let mut tab_container = TabContainer::new(
        bounds,
        TabContainerToolbar::content_bounds(bounds),
        groups,
        view.selected_id,
        TabContainerPlacement::Body,
        crate::WorkbenchUiStyle::from_theme(palette),
        view.dispatch,
    )
    .with_viewport(viewport);
    if let Some(tab) = view
        .visible_action_bar_tab
        .and_then(|tab| mounted_tab_element_id(view.tab_part, tab, TabContainerPlacement::Body))
    {
        tab_container = tab_container.with_visible_action_bar(tab);
    }
    context.draw_component(&tab_container);
    view.dispatch
        .is_focused(SESSION_SEARCH_INPUT)
        .then_some(search_caret)
        .flatten()
}

fn draw_sash(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    orientation: SashOrientation,
    state: SashState,
    identity: ElementId,
    name: &'static str,
    label: &'static str,
    palette: UiTheme,
) {
    let sash = Sash::new(bounds, orientation, state, SashStyle::new(palette.accent));
    context.draw_component(
        &InteractionRegion::new(
            name,
            identity,
            sash.interaction_bounds(),
            AccessibilityRole::Separator,
            label,
        )
        .with_parent(WINDOW)
        .with_cursor(CursorFeedback::ResizeHorizontal)
        .with_value(format!("{} pixels", bounds.origin.x.round())),
    );
    context.draw_component(&sash);
}

fn draw_main(
    context: &mut ComponentContext<'_, '_>,
    layout: WorkbenchSceneLayout,
    view: MainPresentationView<'_>,
    palette: UiTheme,
    text_layout: &mut TextInputLayoutEngine,
) -> MainDrawResult {
    let active_screen = match view.main_surface {
        MainSurfaceKind::Terminal => ScreenBuffer::Alternate,
        MainSurfaceKind::Agent | MainSurfaceKind::Editor => ScreenBuffer::Primary,
    };
    let main_label = if view
        .active_tab_input
        .is_some_and(|input| input.is_settings())
    {
        "Settings"
    } else {
        match view.main_surface {
            MainSurfaceKind::Agent => "Agent",
            MainSurfaceKind::Editor => "Agent session with file inspector",
            MainSurfaceKind::Terminal => "Interactive terminal",
        }
    };
    let main_surface = InteractionRegion::new(
        "MainSurface",
        MAIN_SURFACE,
        layout.main(),
        AccessibilityRole::Group,
        main_label,
    )
    .with_parent(WINDOW)
    .with_cursor(CursorFeedback::Text);
    context.with_component(&main_surface, |context, _| {
        context
            .scene_mut()
            .draw_rect(PaintRect::new(layout.main(), palette.workbench_background));
        context.with_clip(layout.main(), |context| {
            let mut ime_cursor_area = None;
            let mut remote_connection_manager_scroll_metrics = None;
            let mut remote_connection_manager_list_viewport = None;
            if view
                .active_tab_input
                .is_some_and(|input| input.is_settings())
            {
                let platform = view.keybindings.platform();
                let keybinding_rows =
                    zeta_settings::settings_keybinding_rows(platform, |command| {
                        view.keybindings.binding_for_command(command)
                    });
                let surface_label = match view.main_surface {
                    MainSurfaceKind::Agent => "Agent",
                    MainSurfaceKind::Editor => "Editor",
                    MainSurfaceKind::Terminal => "Terminal",
                };
                let theme_scheme =
                    match view.theme_scheme {
                        zeta_theme::ColorScheme::Dark
                        | zeta_theme::ColorScheme::HighContrastDark => "Dark",
                        zeta_theme::ColorScheme::Light
                        | zeta_theme::ColorScheme::HighContrastLight => "Light",
                    };
                let draw = zeta_settings::draw_settings_pane(
                    context,
                    layout.main(),
                    TITLEBAR_HEIGHT,
                    MAIN_SURFACE,
                    SettingsPaneView {
                        state: view.settings,
                        features: SettingsFeatureSnapshot {
                            general: GeneralSettingsSnapshot {
                                directory_label: view.environment_context.working_directory,
                                connection_label: view.environment_context.location,
                                surface_label,
                            },
                            appearance: AppearanceSettingsSnapshot {
                                scheme: theme_scheme,
                                follows_system: view.theme_follows_system,
                            },
                            keybindings: KeybindingSettingsSnapshot {
                                keybinding_rows: &keybinding_rows,
                                keybinding_diagnostics: view.keybinding_diagnostics,
                            },
                            remote: RemoteSettingsSnapshot {
                                connection_manager: view.remote_connection_manager,
                            },
                        },
                        caret_visibility: view.caret_visibility,
                        dispatch: view.dispatch,
                    },
                    SettingsPaneStyle::new(
                        zeta_settings::SettingsPageStyle::from_theme(palette),
                        zeta_settings::SettingsSectionStyle::from_theme(palette),
                        zeta_settings::RemoteUiStyle::from_theme(palette),
                    ),
                    text_layout,
                );
                ime_cursor_area = draw.ime_cursor_area;
                remote_connection_manager_scroll_metrics = draw.remote_connection_scroll_metrics;
                remote_connection_manager_list_viewport = draw.remote_connection_list_viewport;
            } else {
                let active_file_input = view.active_pane.filter(|pane| {
                    !matches!(view.main_surface, MainSurfaceKind::Editor)
                        && matches!(pane.kind(), PaneInputKind::Files | PaneInputKind::Diff)
                });
                if let Some(pane) = active_file_input {
                    let pane_group_id = pane_group_element_id(pane.pane_id());
                    let pane_group = InteractionRegion::new(
                        "PaneGroup",
                        pane_group_id,
                        layout.main(),
                        AccessibilityRole::Group,
                        match pane.kind() {
                            PaneInputKind::Files => "Files pane group",
                            PaneInputKind::Diff => "Changes pane group",
                            _ => unreachable!("file input kind was checked above"),
                        },
                    )
                    .with_parent(MAIN_SURFACE);
                    ime_cursor_area =
                        context.with_component(&pane_group, |context, _| match pane.kind() {
                            PaneInputKind::Files => draw_files_pane(
                                context,
                                layout.main(),
                                view.files,
                                &view.environment_context,
                                pane_group_id,
                                view.caret_visibility,
                                view.dispatch,
                                text_layout,
                                palette,
                            ),
                            PaneInputKind::Diff => {
                                draw_changes_pane(
                                    context,
                                    layout.main(),
                                    view.scm,
                                    pane_group_id,
                                    palette,
                                );
                                None
                            }
                            _ => unreachable!("file input kind was checked above"),
                        });
                } else {
                    match view.main_surface {
                        MainSurfaceKind::Terminal => {
                            let terminal_bounds = terminal_content_bounds(layout, active_screen);
                            if view.terminal_panes.is_empty() {
                                let terminal_region = InteractionRegion::new(
                                    "TerminalOutput",
                                    TERMINAL_OUTPUT,
                                    terminal_bounds,
                                    AccessibilityRole::Terminal,
                                    "Interactive terminal",
                                )
                                .with_parent(MAIN_SURFACE)
                                .with_cursor(CursorFeedback::Text);
                                context.with_component(&terminal_region, |context, _| {
                                    draw_terminal(
                                        context.scene_mut(),
                                        layout,
                                        view.terminal,
                                        active_screen,
                                        palette,
                                    );
                                });
                            } else if let Some(group) = view.pane_group {
                                let pane_geometry =
                                    PaneGroupLayout::for_tree(terminal_bounds, group.tree());
                                for pane in view.terminal_panes {
                                    if pane.kind != PaneInputKind::Terminal {
                                        continue;
                                    }
                                    let Some(pane_id) = pane.pane_id else {
                                        continue;
                                    };
                                    let Some(bounds) =
                                        pane_geometry.leaf(pane_id).map(|leaf| leaf.bounds())
                                    else {
                                        continue;
                                    };
                                    let terminal_region = InteractionRegion::new(
                                        "TerminalPane",
                                        pane_group_element_id(pane_id),
                                        bounds,
                                        AccessibilityRole::Terminal,
                                        "Interactive terminal Pane",
                                    )
                                    .with_parent(MAIN_SURFACE)
                                    .with_cursor(CursorFeedback::Text);
                                    context.with_component(&terminal_region, |context, _| {
                                        draw_terminal_in_bounds(
                                            context.scene_mut(),
                                            bounds,
                                            *pane,
                                            active_screen,
                                            palette,
                                        );
                                    });
                                }
                                context.draw_component(&PanePartSashes::new(
                                    &pane_geometry,
                                    MAIN_SURFACE,
                                    palette.border,
                                    palette.accent,
                                    view.dispatch,
                                    view.terminal_pane_resize_split,
                                ));
                            }
                        }
                        MainSurfaceKind::Agent | MainSurfaceKind::Editor => {
                            ime_cursor_area = draw_session_pane(
                                context,
                                layout.session_pane_layout,
                                SessionPaneView {
                                    title: view.session_title,
                                    state: view.session_pane,
                                    context: view.session_pane_context,
                                    caret_visibility: view.caret_visibility,
                                    dispatch: view.dispatch,
                                    parent: MAIN_SURFACE,
                                },
                                text_layout,
                                zeta_session::SessionPaneStyle::from_theme(palette),
                            );
                        }
                    }
                }
            }
            let ime_cursor_area = if view
                .active_tab_input
                .is_some_and(|input| input.is_settings())
            {
                ime_cursor_area
            } else {
                match view.main_surface {
                    MainSurfaceKind::Terminal if !view.terminal_panes.is_empty() => {
                        let active_pane = view.pane_group.map(PanePart::active_pane);
                        let terminal_bounds = terminal_content_bounds(layout, active_screen);
                        view.terminal_panes
                            .iter()
                            .find(|pane| pane.pane_id == active_pane)
                            .and_then(|pane| {
                                pane.core.and_then(|terminal| {
                                    terminal_cursor_area_for_bounds(
                                        terminal_bounds_for_pane(
                                            view.pane_group,
                                            terminal_bounds,
                                            pane.pane_id,
                                        )?,
                                        terminal,
                                        pane.scroll_offset,
                                    )
                                })
                            })
                    }
                    MainSurfaceKind::Terminal => view.terminal.core.and_then(|terminal| {
                        terminal_cursor_area(layout, terminal, view.terminal.scroll_offset)
                    }),
                    MainSurfaceKind::Agent | MainSurfaceKind::Editor => ime_cursor_area,
                }
            };
            MainDrawResult {
                ime_cursor_area,
                remote_connection_manager_scroll_metrics,
                remote_connection_manager_list_viewport,
            }
        })
    })
}

fn terminal_content_bounds(layout: WorkbenchSceneLayout, active_screen: ScreenBuffer) -> Rect {
    let viewport = if active_screen == ScreenBuffer::Alternate {
        layout.main()
    } else {
        layout.output
    };
    zeta_terminal_runtime::content_bounds(viewport)
}

fn terminal_cursor_area(
    layout: WorkbenchSceneLayout,
    terminal: &TerminalCore,
    scroll_offset: usize,
) -> Option<Rect> {
    let bounds = terminal_content_bounds(layout, ScreenBuffer::Alternate);
    terminal_cursor_area_for_bounds(bounds, terminal, scroll_offset)
}

fn terminal_cursor_area_for_bounds(
    bounds: Rect,
    terminal: &TerminalCore,
    scroll_offset: usize,
) -> Option<Rect> {
    zeta_terminal_runtime::cursor_area(bounds, terminal, scroll_offset)
}

fn terminal_bounds_for_pane(
    group: Option<&PanePart>,
    bounds: Rect,
    pane_id: Option<PaneId>,
) -> Option<Rect> {
    let pane_id = pane_id?;
    PaneGroupLayout::for_tree(bounds, group?.tree())
        .leaf(pane_id)
        .map(|leaf| leaf.bounds())
}

pub fn terminal_pane_sash_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    tab_container: TabContainerState,
    inspector_part: InspectorPartState,
    group: &PanePart,
    point: zui::ui::Point,
) -> Option<(PaneSplitId, SplitViewOrientation, SplitViewResizeSnapshot)> {
    let layout = WorkbenchSceneLayout::for_viewport(viewport, tab_container, inspector_part)?;
    let pane_geometry =
        PaneGroupLayout::for_tree(terminal_content_bounds(layout, active_screen), group.tree());
    pane_geometry.sashes().iter().find_map(|sash| {
        let orientation = match sash.orientation() {
            SplitViewOrientation::Horizontal => SashOrientation::Vertical,
            SplitViewOrientation::Vertical => SashOrientation::Horizontal,
        };
        let component = Sash::new(
            sash.track_bounds(),
            orientation,
            SashState::Resting,
            SashStyle::new(Color::TRANSPARENT),
        );
        component.interaction_bounds().contains(point).then_some((
            sash.split_id(),
            sash.orientation(),
            sash.resize_snapshot(),
        ))
    })
}

fn draw_terminal(
    scene: &mut UiScene,
    layout: WorkbenchSceneLayout,
    view: PaneView<'_>,
    active_screen: ScreenBuffer,
    palette: UiTheme,
) {
    let bounds = terminal_content_bounds(layout, active_screen);
    draw_terminal_in_bounds(scene, bounds, view, active_screen, palette);
}

fn draw_terminal_in_bounds(
    scene: &mut UiScene,
    bounds: Rect,
    view: PaneView<'_>,
    active_screen: ScreenBuffer,
    palette: UiTheme,
) {
    zeta_terminal_runtime::draw_terminal(
        scene,
        bounds,
        zeta_terminal_runtime::TerminalPaneView::new(view.core).with_view_state(
            view.scroll_offset,
            view.scrollbar_presentation,
            view.selection,
        ),
        active_screen,
        palette,
    );
}

fn draw_compact_scene(
    scene: &mut UiScene,
    viewport: LogicalViewport,
    product_name: &str,
    palette: UiTheme,
) {
    let bounds = Rect::from_xywh(
        12.0,
        12.0,
        (viewport.width - 24.0).max(1.0),
        (viewport.height - 24.0).max(1.0),
    );
    scene.draw_rect(
        PaintRect::new(bounds, palette.content_background)
            .with_border(Border::uniform(1.0, palette.border))
            .with_corner_radii(CornerRadii::uniform(10.0)),
    );
    draw_text(
        scene,
        product_name,
        Rect::from_xywh(
            bounds.origin.x + 18.0,
            bounds.origin.y + 18.0,
            (bounds.size.width - 36.0).max(1.0),
            30.0,
        ),
        TextStyle::new(20.0, palette.foreground).with_weight(FontWeight::Bold),
    );
}

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    if bounds.is_empty() {
        return;
    }
    scene.draw_text(TextBlock::new(text, bounds.origin, bounds.size, style));
}
