//! Compatibility exports for the Session UI timeline.

use crate::shell_style::ShellPalette;

pub(crate) use zeta_session_ui::ThreadTimeline;
pub(crate) use zeta_session_ui::line_capacity;
pub(crate) use zeta_session_ui::line_count;

pub(crate) fn thread_timeline_style(palette: ShellPalette) -> zeta_session_ui::ThreadTimelineStyle {
    zeta_session_ui::ThreadTimelineStyle::new(
        palette.surface_raised,
        palette.text,
        palette.text_muted,
        palette.error,
    )
}
