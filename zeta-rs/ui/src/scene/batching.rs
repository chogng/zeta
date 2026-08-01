use std::ops::Range;

/// One consecutive batch of same-kind primitives in resolved scene paint order.
///
/// Backends should consume batches in iterator order. Ranges index the corresponding primitive
/// slice on [`super::UiScene`]; adjacent calls of the same kind are coalesced, while a change of
/// kind or composition layer starts a new batch so visual stacking remains exact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SceneBatch {
    Rects { layer: usize, range: Range<usize> },
    Icons { layer: usize, range: Range<usize> },
    Images { layer: usize, range: Range<usize> },
    Text { layer: usize, range: Range<usize> },
}

impl SceneBatch {
    /// Returns the ordered composition layer containing this batch.
    pub const fn layer(&self) -> usize {
        match self {
            Self::Rects { layer, .. }
            | Self::Icons { layer, .. }
            | Self::Images { layer, .. }
            | Self::Text { layer, .. } => *layer,
        }
    }

    /// Returns the primitive range in the batch's corresponding scene slice.
    pub fn range(&self) -> Range<usize> {
        match self {
            Self::Rects { range, .. }
            | Self::Icons { range, .. }
            | Self::Images { range, .. }
            | Self::Text { range, .. } => range.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ScenePrimitive {
    Rect(usize),
    Icon(usize),
    Image(usize),
    Text(usize),
}

impl ScenePrimitive {
    const fn kind(self) -> ScenePrimitiveKind {
        match self {
            Self::Rect(_) => ScenePrimitiveKind::Rect,
            Self::Icon(_) => ScenePrimitiveKind::Icon,
            Self::Image(_) => ScenePrimitiveKind::Image,
            Self::Text(_) => ScenePrimitiveKind::Text,
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Rect(index) | Self::Icon(index) | Self::Image(index) | Self::Text(index) => index,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScenePrimitiveKind {
    Rect,
    Icon,
    Image,
    Text,
}

pub(super) fn batches(
    layer_primitives: &[Vec<ScenePrimitive>],
) -> impl Iterator<Item = SceneBatch> + '_ {
    let mut layer = 0;
    let mut offset = 0;
    std::iter::from_fn(move || {
        loop {
            let operations = layer_primitives.get(layer)?;
            let Some(first) = operations.get(offset).copied() else {
                layer += 1;
                offset = 0;
                continue;
            };
            let kind = first.kind();
            let start = first.index();
            let mut previous_index = start;
            offset += 1;
            while operations.get(offset).is_some_and(|operation| {
                operation.kind() == kind && operation.index() == previous_index + 1
            }) {
                previous_index += 1;
                offset += 1;
            }
            let range = start..previous_index + 1;
            return Some(match kind {
                ScenePrimitiveKind::Rect => SceneBatch::Rects { layer, range },
                ScenePrimitiveKind::Icon => SceneBatch::Icons { layer, range },
                ScenePrimitiveKind::Image => SceneBatch::Images { layer, range },
                ScenePrimitiveKind::Text => SceneBatch::Text { layer, range },
            });
        }
    })
}
