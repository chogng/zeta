use std::time::Duration;
use std::time::Instant;

use super::TestEvent;
use super::TestRuntime;
use super::TestTimerScope;
use crate::ui::foundation::Color;
use crate::ui::presentation::UiScene;
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
    let scene = UiScene::new(Color::rgb(10, 20, 30));

    runtime.present_scene(window, &scene, &[]).unwrap();

    let window = runtime.window(window).unwrap();
    assert_eq!(window.title(), "Preview");
    assert_eq!(window.logical_size(), LogicalSize::new(800.0, 600.0));
    assert_eq!(window.renderer().state().scenes(), &[scene]);
    assert!(window.accessibility().is_empty());
}
