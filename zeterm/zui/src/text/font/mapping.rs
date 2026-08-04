use cosmic_text::{Family, Style, Weight};

use crate::text::FontFamily;
use crate::text::FontStyle;
use crate::text::FontWeight;

pub(crate) fn shaping_family(family: &FontFamily) -> Family<'_> {
    match family {
        FontFamily::SansSerif => Family::SansSerif,
        FontFamily::Serif => Family::Serif,
        FontFamily::Monospace => Family::Monospace,
        FontFamily::Named(name) => Family::Name(name),
    }
}

pub(crate) fn shaping_weight(weight: FontWeight) -> Weight {
    match weight {
        FontWeight::Normal => Weight::NORMAL,
        FontWeight::Bold => Weight::BOLD,
    }
}

pub(crate) fn shaping_style(style: FontStyle) -> Style {
    match style {
        FontStyle::Normal => Style::Normal,
        FontStyle::Italic => Style::Italic,
    }
}
