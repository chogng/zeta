use crate::CommandRegistry;
use crate::CommandRegistryError;
use crate::CommandRequest;
use crate::ZetermCommandId;

#[derive(Default)]
struct Context {
    executed: Vec<ZetermCommandId>,
}

fn record_command(context: &mut Context, request: &CommandRequest) {
    context.executed.push(request.command_id());
}

#[test]
fn registered_handler_executes_through_command_request() {
    let mut registry = CommandRegistry::new();
    registry
        .register(ZetermCommandId::Save, record_command)
        .expect("Save should be registered once");

    let request = CommandRequest::new(ZetermCommandId::Save);
    let mut context = Context::default();
    registry
        .execute(&mut context, &request)
        .expect("registered command should execute");

    assert_eq!(context.executed, [ZetermCommandId::Save]);
}

#[test]
fn registry_rejects_duplicate_and_missing_handlers() {
    let mut registry = CommandRegistry::new();
    registry
        .register(ZetermCommandId::Save, record_command)
        .expect("Save should be registered once");
    assert_eq!(
        registry.register(ZetermCommandId::Save, record_command),
        Err(CommandRegistryError::AlreadyRegistered(
            ZetermCommandId::Save
        ))
    );

    let mut context = Context::default();
    let request = CommandRequest::new(ZetermCommandId::Paste);
    assert_eq!(
        registry.execute(&mut context, &request),
        Err(CommandRegistryError::NotRegistered(ZetermCommandId::Paste))
    );
}

#[test]
fn command_request_converts_from_command_id() {
    let request = CommandRequest::from(ZetermCommandId::Copy);
    assert_eq!(request.command_id(), ZetermCommandId::Copy);
}
