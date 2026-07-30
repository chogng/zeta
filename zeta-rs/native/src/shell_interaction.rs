use zeta_ui::{Point, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShellTarget {
    WindowDrag,
    Composer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PointerFeedback {
    Default,
    Text,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InteractionEffect {
    None,
    Redraw,
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

    pub(crate) fn target_at(&self, point: Point) -> Option<ShellTarget> {
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
}

impl ShellInteraction {
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
            Some(ShellTarget::Composer) | None => InteractionEffect::None,
        }
    }

    pub(crate) fn release_primary(&mut self) -> InteractionEffect {
        InteractionEffect::None
    }

    pub(crate) const fn pointer_feedback(&self) -> PointerFeedback {
        match self.hovered {
            Some(ShellTarget::WindowDrag) => PointerFeedback::Default,
            Some(ShellTarget::Composer) | None => PointerFeedback::Text,
        }
    }
}

#[cfg(test)]
#[path = "shell_interaction_tests.rs"]
mod tests;
