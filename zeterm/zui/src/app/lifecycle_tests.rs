use super::ExitPolicy;
use super::WindowCommand;
use super::WindowCommandQueue;

#[test]
fn exit_policy_defaults_to_closing_with_the_last_window() {
    assert_eq!(ExitPolicy::default(), ExitPolicy::OnLastWindowClosed);
}

#[test]
fn window_commands_preserve_callback_order() {
    let mut commands = WindowCommandQueue::default();
    commands.push(WindowCommand::Exit);
    commands.push(WindowCommand::Exit);

    assert_eq!(commands.pop(), Some(WindowCommand::Exit));
    assert_eq!(commands.pop(), Some(WindowCommand::Exit));
    assert_eq!(commands.pop(), None);
}
