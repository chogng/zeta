use std::time::Duration;
use std::time::Instant;

use super::TestEvent;
use super::TestRuntime;
use super::TestTimerScope;
use crate::runtime::AccessibilityRole;
use crate::runtime::FocusBehavior;
use crate::runtime::InteractionFrame;
use crate::runtime::UiDispatch;
use crate::runtime::UiNode;
use crate::ui::foundation::Color;
use crate::ui::foundation::ElementId;
use crate::ui::foundation::Rect;
use crate::ui::presentation::UiFrame;
use crate::window::LogicalSize;

#[test]
fn lifecycle_and_timer_events_follow_deterministic_fifo_order() {
    let mut runtime = TestRuntime::at(Instant::now());
    runtime.resume();
    let first = runtime.open_window("First", LogicalSize::new(640.0, 480.0));
    let second = runtime.open_window("Second", LogicalSize::new(320.0, 240.0));
    runtime.request_redraw(first);
    runtime.schedule_after(
        TestTimerScope::Window(second),
        Duration::from_millis(5),
        "cancelled",
    );
    runtime.schedule_after(
        TestTimerScope::Application,
        Duration::from_millis(10),
        "delivered",
    );
    runtime.close_window(second);
    runtime.advance(Duration::from_millis(10));
    runtime.close_window(first);

    assert_eq!(runtime.next_event(), Some(TestEvent::Resumed));
    assert_eq!(runtime.next_event(), Some(TestEvent::WindowOpened(first)));
    assert_eq!(runtime.next_event(), Some(TestEvent::WindowOpened(second)));
    assert_eq!(
        runtime.next_event(),
        Some(TestEvent::RedrawRequested(first))
    );
    assert_eq!(runtime.next_event(), Some(TestEvent::WindowClosed(second)));
    assert_eq!(runtime.next_event(), Some(TestEvent::User("delivered")));
    assert_eq!(runtime.next_event(), Some(TestEvent::WindowClosed(first)));
    assert_eq!(runtime.next_event(), Some(TestEvent::Exiting));
    assert_eq!(runtime.next_event(), None);
}

#[test]
fn headless_windows_record_presented_scene_snapshots() {
    let mut runtime = TestRuntime::<()>::at(Instant::now());
    let window = runtime.open_window("Preview", LogicalSize::new(800.0, 600.0));
    let target = ElementId::scoped(1, 1);
    let mut frame = UiFrame::<InteractionFrame>::new(Color::rgb(10, 20, 30));
    frame.interaction_mut().register(
        UiNode::new(
            target,
            Rect::from_xywh(10.0, 20.0, 80.0, 30.0),
            AccessibilityRole::Button,
            "Run",
        )
        .with_focus(FocusBehavior::TabStop),
    );
    let mut dispatch = UiDispatch::default();
    dispatch.focus_element(frame.interaction(), target);

    runtime.present_frame(window, &frame, &dispatch).unwrap();

    let window = runtime.window(window).unwrap();
    assert_eq!(window.title(), "Preview");
    assert_eq!(window.logical_size(), LogicalSize::new(800.0, 600.0));
    assert_eq!(window.renderer().state().scenes(), &[frame.scene().clone()]);
    assert_eq!(window.accessibility().len(), 1);
    assert_eq!(window.accessibility()[0].id, target);
    assert!(window.accessibility()[0].focused);
}
