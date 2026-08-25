use crate::input::ElementState;
use crate::input::Key;
use crate::input::KeyCode;
use crate::input::KeyEvent;
use crate::input::ModifiersState;
use crate::input::NamedKey;
use crate::input::PhysicalKey;
use crate::window::PhysicalExtent;
use crate::window::WindowEvent;

#[test]
fn key_event_builders_create_backend_independent_test_input() {
    let event = KeyEvent::new(Key::Character("z".to_owned()), ElementState::Pressed)
        .with_physical_key(PhysicalKey::Code(KeyCode::new("KeyZ")))
        .with_text("z")
        .repeated();

    assert_eq!(event.logical_key, Key::Character("z".to_owned()));
    assert_eq!(format!("{:?}", event.physical_key), "Code(KeyZ)");
    assert_eq!(event.text.as_deref(), Some("z"));
    assert!(event.repeat);
}

#[test]
fn modifiers_are_constructible_without_a_window_backend() {
    let modifiers = ModifiersState::default()
        .with_control()
        .with_shift()
        .with_alt()
        .with_super();

    assert!(modifiers.control_key());
    assert!(modifiers.shift_key());
    assert!(modifiers.alt_key());
    assert!(modifiers.super_key());
}

#[test]
fn native_window_events_are_normalized_before_application_dispatch() {
    let resized = WindowEvent::from_native(winit::event::WindowEvent::Resized(
        winit::dpi::PhysicalSize::new(1280, 800),
    ));
    let unknown_key = NamedKey::from_native(winit::keyboard::NamedKey::AudioVolumeUp);

    assert_eq!(
        resized,
        WindowEvent::Resized(PhysicalExtent::new(1280, 800))
    );
    assert_eq!(format!("{unknown_key:?}"), "AudioVolumeUp");
}
