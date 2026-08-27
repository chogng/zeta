use zui::ui::AnimationKey;
use zui::ui::AnimationProperty;
use zui::ui::ElementId;

const FOLD_SCOPE: u32 = 4;
const SECTION_SCOPE: u32 = 10;
const HEADER_SCOPE: u32 = 11;
const DIFF_SCOPE: u32 = 12;

/// Stable identity for one changed-file section in a [`MultiDiffEditor`].
///
/// The product host allocates a slot from the lifetime of the represented file, not from its
/// current position in a snapshot. The same identity derives the section, header, diff body,
/// fold controls, and height-animation track, so reordering files does not move retained state to
/// another file. Slots must be non-zero and remain stable while the file is retained by the host.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MultiDiffEditorItemIdentity {
    slot: u32,
}

impl MultiDiffEditorItemIdentity {
    /// Creates an identity from a host-owned non-zero slot.
    ///
    /// A slot is intentionally opaque to the editor presentation. Hosts should allocate it from
    /// a stable file key and must not derive it from the file's current array index.
    pub fn from_slot(slot: u32) -> Self {
        assert!(slot != 0, "multi-diff item identity slots must be non-zero");
        Self { slot }
    }

    /// Returns the host-owned slot for diagnostics and deterministic tests.
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// Returns the identity of the file card that contains the diff.
    pub const fn section_id(self) -> ElementId {
        ElementId::scoped(SECTION_SCOPE, self.slot)
    }

    /// Returns the identity of the file header inside the card.
    pub const fn header_id(self) -> ElementId {
        ElementId::scoped(HEADER_SCOPE, self.slot)
    }

    /// Returns the identity of the file's nested diff body.
    pub const fn diff_id(self) -> ElementId {
        ElementId::scoped(DIFF_SCOPE, self.slot)
    }

    /// Returns the identity of an unchanged-region fold control.
    ///
    /// The legacy fold encoding reserves 16 bits for the item slot and region index. Keeping that
    /// encoding lets product input routing migrate without making existing IDs ambiguous.
    pub fn fold_id(self, region_index: usize) -> Option<ElementId> {
        let slot = u16::try_from(self.slot).ok()?;
        let region_index = u16::try_from(region_index).ok()?;
        let local = ((u32::from(slot) << 16) | u32::from(region_index)).checked_add(1)?;
        Some(ElementId::scoped(FOLD_SCOPE, local))
    }

    /// Returns the stable animation key for this file card's fold-driven height.
    pub const fn fold_animation_key(self) -> AnimationKey {
        AnimationKey::new(self.section_id(), AnimationProperty::Height)
    }
}
