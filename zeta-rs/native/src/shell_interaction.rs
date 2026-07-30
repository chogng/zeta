use zeta_ui::{Point, Rect, TextInput, TextInputCommand, TextInputCompositionEvent};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SessionId {
    #[default]
    Foundation,
    Renderer,
    AppServer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShellTarget {
    WindowDrag,
    Session(SessionId),
    Composer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PointerFeedback {
    Default,
    Clickable,
    Text,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InteractionEffect {
    None,
    Redraw,
    FocusComposer,
    BlurComposer,
    StartWindowDrag,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HitRegion {
    bounds: Rect,
    target: ShellTarget,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ShellHitMap {
    regions: Vec<HitRegion>,
}

impl ShellHitMap {
    pub(crate) fn register(&mut self, bounds: Rect, target: ShellTarget) {
        self.regions.push(HitRegion { bounds, target });
    }

    fn target_at(&self, point: Point) -> Option<ShellTarget> {
        self.regions
            .iter()
            .rev()
            .find(|region| region.bounds.contains(point))
            .map(|region| region.target)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ShellInteraction {
    hovered: Option<ShellTarget>,
    pressed: Option<ShellTarget>,
    selected_session: SessionId,
    composer_focused: bool,
    composer: TextInput,
}

impl ShellInteraction {
    pub(crate) const fn selected_session(&self) -> SessionId {
        self.selected_session
    }

    pub(crate) const fn composer_focused(&self) -> bool {
        self.composer_focused
    }

    pub(crate) const fn composer(&self) -> &TextInput {
        &self.composer
    }

    pub(crate) fn is_hovered(&self, target: ShellTarget) -> bool {
        self.hovered == Some(target)
    }

    pub(crate) fn is_pressed(&self, target: ShellTarget) -> bool {
        self.pressed == Some(target) && self.hovered == Some(target)
    }

    pub(crate) fn pointer_moved(
        &mut self,
        point: Point,
        hit_map: &ShellHitMap,
    ) -> InteractionEffect {
        let hovered = hit_map.target_at(point);
        if self.hovered == hovered {
            return InteractionEffect::None;
        }
        self.hovered = hovered;
        InteractionEffect::Redraw
    }

    pub(crate) fn pointer_left(&mut self) -> InteractionEffect {
        if self.hovered.take().is_some() {
            InteractionEffect::Redraw
        } else {
            InteractionEffect::None
        }
    }

    pub(crate) fn press_primary(&mut self) -> InteractionEffect {
        match self.hovered {
            Some(ShellTarget::WindowDrag) => InteractionEffect::StartWindowDrag,
            target => {
                let focus_changed = self.composer_focused && target != Some(ShellTarget::Composer);
                if focus_changed {
                    self.composer_focused = false;
                    self.composer.cancel_composition();
                }
                self.pressed = target;
                if focus_changed {
                    InteractionEffect::BlurComposer
                } else if target.is_some() {
                    InteractionEffect::Redraw
                } else {
                    InteractionEffect::None
                }
            }
        }
    }

    pub(crate) fn release_primary(&mut self) -> InteractionEffect {
        let pressed = self.pressed.take();
        let activated = pressed.filter(|target| Some(*target) == self.hovered);
        match activated {
            Some(ShellTarget::Session(session)) => {
                self.selected_session = session;
                self.composer_focused = false;
            }
            Some(ShellTarget::Composer) if !self.composer_focused => {
                self.composer_focused = true;
                return InteractionEffect::FocusComposer;
            }
            Some(ShellTarget::Composer) => {}
            Some(ShellTarget::WindowDrag) | None => {}
        };
        if pressed.is_some() || activated.is_some() {
            InteractionEffect::Redraw
        } else {
            InteractionEffect::None
        }
    }

    pub(crate) fn edit_composer(&mut self, command: TextInputCommand) -> InteractionEffect {
        if !self.composer_focused {
            return InteractionEffect::None;
        }
        self.composer.apply(command);
        InteractionEffect::Redraw
    }

    pub(crate) fn update_composition(
        &mut self,
        event: TextInputCompositionEvent,
    ) -> InteractionEffect {
        if !self.composer_focused {
            return InteractionEffect::None;
        }
        self.composer.apply_composition(event);
        InteractionEffect::Redraw
    }

    pub(crate) fn window_focus_lost(&mut self) -> InteractionEffect {
        if !self.composer_focused {
            return InteractionEffect::None;
        }
        self.composer_focused = false;
        self.composer.cancel_composition();
        InteractionEffect::BlurComposer
    }

    pub(crate) const fn pointer_feedback(&self) -> PointerFeedback {
        match self.hovered {
            Some(ShellTarget::Session(_)) => PointerFeedback::Clickable,
            Some(ShellTarget::Composer) => PointerFeedback::Text,
            Some(ShellTarget::WindowDrag) | None => PointerFeedback::Default,
        }
    }
}

#[cfg(test)]
#[path = "shell_interaction_tests.rs"]
mod tests;
