use glyphon::{Family, Style, Weight};

use crate::{FontFamily, FontStyle, FontWeight};

pub(crate) fn glyphon_family(family: &FontFamily) -> Family<'_> {
    match family {
        FontFamily::SansSerif => Family::SansSerif,
        FontFamily::Serif => Family::Serif,
        FontFamily::Monospace => Family::Monospace,
        FontFamily::Named(name) => Family::Name(name),
    }
}

pub(crate) fn glyphon_weight(weight: FontWeight) -> Weight {
    match weight {
        FontWeight::Normal => Weight::NORMAL,
        FontWeight::Bold => Weight::BOLD,
    }
}

pub(crate) fn glyphon_style(style: FontStyle) -> Style {
    match style {
        FontStyle::Normal => Style::Normal,
        FontStyle::Italic => Style::Italic,
    }
}
