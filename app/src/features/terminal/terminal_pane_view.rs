use crate::terminal_pointer::TerminalPointer;
use crate::terminal_scrollback::TerminalScroll;
use crate::terminal_selection::TerminalSelection;

/// Ephemeral view state retained independently for one terminal Pane.
#[derive(Default)]
pub(crate) struct TerminalPaneViewState {
    pub(crate) scroll: TerminalScroll,
    pub(crate) pointer: TerminalPointer,
    pub(crate) selection: TerminalSelection,
}
