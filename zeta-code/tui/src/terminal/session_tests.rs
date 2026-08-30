use super::TerminalModeGuard;
use super::TerminalModeOperations;
use crate::mouse::MouseMode;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

const ENABLE_RAW_MODE: &str = "enable raw mode";
const ENTER_ALTERNATE_SCREEN: &str = "enter alternate screen";
const ENABLE_BRACKETED_PASTE: &str = "enable bracketed paste";
const ENABLE_MOUSE_CAPTURE: &str = "enable mouse capture";
const DISABLE_MOUSE_CAPTURE: &str = "disable mouse capture";
const DISABLE_BRACKETED_PASTE: &str = "disable bracketed paste";
const LEAVE_ALTERNATE_SCREEN: &str = "leave alternate screen";
const DISABLE_RAW_MODE: &str = "disable raw mode";

#[test]
fn acquired_terminal_modes_are_restored_in_reverse_order() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            ENABLE_BRACKETED_PASTE,
            DISABLE_BRACKETED_PASTE,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

#[test]
fn acquisition_failure_restores_only_modes_that_were_acquired() {
    let cases = [
        (ENABLE_RAW_MODE, vec![ENABLE_RAW_MODE]),
        (
            ENTER_ALTERNATE_SCREEN,
            vec![ENABLE_RAW_MODE, ENTER_ALTERNATE_SCREEN, DISABLE_RAW_MODE],
        ),
        (
            ENABLE_BRACKETED_PASTE,
            vec![
                ENABLE_RAW_MODE,
                ENTER_ALTERNATE_SCREEN,
                ENABLE_BRACKETED_PASTE,
                LEAVE_ALTERNATE_SCREEN,
                DISABLE_RAW_MODE,
            ],
        ),
    ];

    for (failure, expected_calls) in cases {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let result = TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), Some(failure)));

        assert!(result.is_err(), "{failure} should fail");
        assert_eq!(
            calls.borrow().as_slice(),
            expected_calls,
            "unexpected rollback for {failure}"
        );
    }
}

#[test]
fn mouse_mode_is_applied_idempotently() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    guard
        .set_mouse_mode(MouseMode::TuiCapture)
        .expect("enable TUI pointer capture");
    guard
        .set_mouse_mode(MouseMode::TuiCapture)
        .expect("keep TUI pointer capture enabled");
    guard
        .set_mouse_mode(MouseMode::TerminalSelection)
        .expect("restore terminal selection");
    guard
        .set_mouse_mode(MouseMode::TerminalSelection)
        .expect("keep terminal selection restored");
    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_MOUSE_CAPTURE,
            DISABLE_MOUSE_CAPTURE,
            DISABLE_BRACKETED_PASTE,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

#[test]
fn explicit_restore_is_idempotent() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    guard.restore();
    guard.restore();
    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            ENABLE_BRACKETED_PASTE,
            DISABLE_BRACKETED_PASTE,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

#[test]
fn suspend_cycle_reacquires_requested_mouse_capture() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    guard
        .set_mouse_mode(MouseMode::TuiCapture)
        .expect("enable TUI pointer capture");
    guard.restore();
    guard.reacquire().expect("reacquire");
    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_MOUSE_CAPTURE,
            DISABLE_MOUSE_CAPTURE,
            DISABLE_BRACKETED_PASTE,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_MOUSE_CAPTURE,
            DISABLE_MOUSE_CAPTURE,
            DISABLE_BRACKETED_PASTE,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

struct FakeOperations {
    calls: Rc<RefCell<Vec<&'static str>>>,
    failure: Option<&'static str>,
}

impl FakeOperations {
    fn new(calls: Rc<RefCell<Vec<&'static str>>>, failure: Option<&'static str>) -> Self {
        Self { calls, failure }
    }

    fn call(&self, operation: &'static str) -> io::Result<()> {
        self.calls.borrow_mut().push(operation);
        if self.failure == Some(operation) {
            Err(io::Error::other(format!("{operation} failed")))
        } else {
            Ok(())
        }
    }
}

impl TerminalModeOperations for FakeOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        self.call(ENABLE_RAW_MODE)
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        self.call(ENTER_ALTERNATE_SCREEN)
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        self.call(ENABLE_BRACKETED_PASTE)
    }

    fn enable_mouse_capture(&mut self) -> io::Result<()> {
        self.call(ENABLE_MOUSE_CAPTURE)
    }

    fn disable_mouse_capture(&mut self) -> io::Result<()> {
        self.call(DISABLE_MOUSE_CAPTURE)
    }

    fn disable_bracketed_paste(&mut self) -> io::Result<()> {
        self.call(DISABLE_BRACKETED_PASTE)
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        self.call(LEAVE_ALTERNATE_SCREEN)
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        self.call(DISABLE_RAW_MODE)
    }
}
