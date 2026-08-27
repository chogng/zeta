//! Compatibility exports for the Session UI timeline.

use crate::shell_style::ShellPalette;

pub(crate) use zeta_session::ThreadTimeline;
pub(crate) use zeta_session::line_capacity;
pub(crate) use zeta_session::line_count;

pub(crate) fn thread_timeline_style(palette: ShellPalette) -> zeta_session::ThreadTimelineStyle {
    zeta_session::ThreadTimelineStyle::new(
        palette.surface_raised,
        palette.text,
        palette.text_muted,
        palette.error,
    )
}
