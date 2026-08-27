use crate::AppCommandId;
use crate::CommandRegistry;
use crate::CommandRegistryError;
use crate::CommandRequest;

#[derive(Default)]
struct Context {
    executed: Vec<AppCommandId>,
}

fn record_command(context: &mut Context, request: &CommandRequest) {
    context.executed.push(request.command_id());
}

#[test]
fn registered_handler_executes_through_command_request() {
    let mut registry = CommandRegistry::new();
    registry
        .register(AppCommandId::Save, record_command)
        .expect("Save should be registered once");

    let request = CommandRequest::new(AppCommandId::Save);
    let mut context = Context::default();
    registry
        .execute(&mut context, &request)
        .expect("registered command should execute");

    assert_eq!(context.executed, [AppCommandId::Save]);
}

#[test]
fn registry_rejects_duplicate_and_missing_handlers() {
    let mut registry = CommandRegistry::new();
    registry
        .register(AppCommandId::Save, record_command)
        .expect("Save should be registered once");
    assert_eq!(
        registry.register(AppCommandId::Save, record_command),
        Err(CommandRegistryError::AlreadyRegistered(AppCommandId::Save))
    );

    let mut context = Context::default();
    let request = CommandRequest::new(AppCommandId::Paste);
    assert_eq!(
        registry.execute(&mut context, &request),
        Err(CommandRegistryError::NotRegistered(AppCommandId::Paste))
    );
}

#[test]
fn command_request_converts_from_command_id() {
    let request = CommandRequest::from(AppCommandId::Copy);
    assert_eq!(request.command_id(), AppCommandId::Copy);
}
