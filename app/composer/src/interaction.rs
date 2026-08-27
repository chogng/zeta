use zeta_protocol::ModelRef;
use zeta_slash_commands::{
    SlashCommandArgumentMode, SlashCommandCatalog, SlashCommandCatalogError,
    SlashCommandDefinition, SlashCommandsState,
};

use crate::ComposerRoute;

#[derive(Clone, Debug, Eq, PartialEq)]
enum InteractionItemAction {
    CompleteSlash,
    OpenModelPicker,
    SelectModel(ModelRef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerInteractionItem {
    label: String,
    description: String,
    action: InteractionItemAction,
}

impl ComposerInteractionItem {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Composer-facing model option normalized from the product's model catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerModelOption {
    pub label: String,
    pub description: String,
    pub model: ModelRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModelPickerState {
    items: Vec<ComposerInteractionItem>,
    selected: usize,
}

impl ModelPickerState {
    fn new(items: Vec<ComposerInteractionItem>) -> Self {
        Self { items, selected: 0 }
    }

    fn move_selection(&mut self, direction: SelectionDirection) {
        if self.items.is_empty() {
            return;
        }
        self.selected = match direction {
            SelectionDirection::Previous => self.selected.saturating_sub(1),
            SelectionDirection::Next => (self.selected + 1).min(self.items.len() - 1),
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionDirection {
    Previous,
    Next,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComposerInteractionActivation {
    ComposerText(String),
    Model(ModelRef),
    ViewChanged,
}

#[derive(Clone, Copy)]
pub struct ComposerInteractionView<'a> {
    title: &'static str,
    items: &'a [ComposerInteractionItem],
    selected: usize,
    can_go_back: bool,
}

impl<'a> ComposerInteractionView<'a> {
    pub const fn title(self) -> &'static str {
        self.title
    }

    pub const fn items(self) -> &'a [ComposerInteractionItem] {
        self.items
    }

    pub const fn selected(self) -> usize {
        self.selected
    }

    pub const fn can_go_back(self) -> bool {
        self.can_go_back
    }
}

/// Composer-owned model that selects and updates the active interaction view.
///
/// Presentation geometry, clipping, and scrolling belong to the Composer Pane and UI component
/// primitives; this model only owns Slash command and model-picker domain state and exposes an
/// immutable render view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComposerInteractionModel {
    slash_commands: SlashCommandsState,
    slash_items: Vec<ComposerInteractionItem>,
    models: Vec<ComposerInteractionItem>,
    model_picker: Option<ModelPickerState>,
}

impl ComposerInteractionModel {
    pub fn new() -> Self {
        let catalog = SlashCommandCatalog::with_local_and_server([model_command()], [])
            .expect("the native /model command is valid");
        Self {
            slash_commands: SlashCommandsState::new(catalog),
            ..Self::default()
        }
    }

    pub fn set_catalog(
        &mut self,
        slash_commands: Vec<SlashCommandDefinition>,
        models: Vec<ComposerModelOption>,
    ) -> Result<(), SlashCommandCatalogError> {
        let catalog =
            SlashCommandCatalog::with_local_and_server([model_command()], slash_commands)?;
        self.slash_commands.set_catalog(catalog);
        self.models = models
            .into_iter()
            .map(|option| ComposerInteractionItem {
                label: option.label,
                description: option.description,
                action: InteractionItemAction::SelectModel(option.model),
            })
            .collect();
        self.refresh_slash_items();
        Ok(())
    }

    pub fn sync_for_composer(&mut self, text: &str, route: ComposerRoute) {
        if route != ComposerRoute::Agent {
            self.close();
            return;
        }
        if self.model_picker.is_some() {
            return;
        }
        self.slash_commands.sync_input(text, text.len());
        self.refresh_slash_items();
    }

    pub fn is_visible(&self) -> bool {
        self.model_picker.is_some() || self.slash_commands.view().is_some()
    }

    pub fn is_model_picker_visible(&self) -> bool {
        self.model_picker.is_some()
    }

    pub fn view(&self) -> Option<ComposerInteractionView<'_>> {
        if let Some(view) = &self.model_picker {
            return Some(ComposerInteractionView {
                title: "Select model",
                items: &view.items,
                selected: view.selected,
                can_go_back: true,
            });
        }
        let view = self.slash_commands.view()?;
        Some(ComposerInteractionView {
            title: "Commands",
            items: &self.slash_items,
            selected: view.selected,
            can_go_back: false,
        })
    }

    pub fn move_selection(&mut self, direction: SelectionDirection) {
        if let Some(view) = &mut self.model_picker {
            view.move_selection(direction);
            return;
        }
        match direction {
            SelectionDirection::Previous => self.slash_commands.select_previous(),
            SelectionDirection::Next => self.slash_commands.select_next(),
        }
    }

    pub fn select_item(&mut self, index: usize) -> bool {
        if let Some(view) = &mut self.model_picker {
            if index >= view.items.len() {
                return false;
            }
            view.selected = index;
            return true;
        }
        self.slash_commands.select(index)
    }

    pub fn activate_selected(&mut self) -> Option<ComposerInteractionActivation> {
        if let Some(view) = &self.model_picker {
            let action = view.items.get(view.selected)?.action.clone();
            let InteractionItemAction::SelectModel(model) = action else {
                return None;
            };
            self.close();
            return Some(ComposerInteractionActivation::Model(model));
        }
        let command = self.slash_commands.selected_command()?;
        if command.name == "model" {
            self.model_picker = Some(ModelPickerState::new(self.models.clone()));
            return Some(ComposerInteractionActivation::ViewChanged);
        }
        let completion = self.slash_commands.selected_completion()?;
        self.close();
        Some(ComposerInteractionActivation::ComposerText(
            completion.replacement,
        ))
    }

    pub fn complete_selected_slash(&mut self) -> Option<String> {
        if self.model_picker.is_some() {
            return None;
        }
        let completion = self.slash_commands.selected_completion()?;
        self.close();
        Some(completion.replacement)
    }

    pub fn dismiss(&mut self, _composer_text: &str) -> bool {
        if self.model_picker.take().is_some() {
            return true;
        }
        if self.slash_commands.view().is_none() {
            return false;
        }
        self.slash_commands.dismiss();
        self.slash_items.clear();
        true
    }

    fn close(&mut self) {
        self.model_picker = None;
        self.slash_commands.clear();
        self.slash_items.clear();
    }

    fn refresh_slash_items(&mut self) {
        self.slash_items = self
            .slash_commands
            .view()
            .map(|view| {
                view.commands
                    .iter()
                    .map(|command| ComposerInteractionItem {
                        label: format!("/{}", command.name),
                        description: command.description.clone(),
                        action: if command.name == "model" {
                            InteractionItemAction::OpenModelPicker
                        } else {
                            InteractionItemAction::CompleteSlash
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
}

fn model_command() -> SlashCommandDefinition {
    SlashCommandDefinition {
        name: "model".into(),
        description: "choose the model for this session".into(),
        argument_mode: SlashCommandArgumentMode::None,
    }
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
