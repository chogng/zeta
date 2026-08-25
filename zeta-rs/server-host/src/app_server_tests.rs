use super::AppServerHostCommand;
use super::LifecycleCommand;
use super::parse_arguments;

#[test]
fn app_server_commands_select_direct_connect_and_lifecycle_modes() {
    assert_eq!(
        parse_arguments(&["--listen".into(), "stdio://".into()]).unwrap(),
        (AppServerHostCommand::Direct, None)
    );
    assert_eq!(
        parse_arguments(&["connect".into()]).unwrap(),
        (AppServerHostCommand::Connect, None)
    );
    assert_eq!(
        ["start", "restart", "stop", "version"].map(|command| {
            parse_arguments(&["daemon".into(), command.into()])
                .unwrap()
                .0
        }),
        [
            AppServerHostCommand::Daemon(LifecycleCommand::Start),
            AppServerHostCommand::Daemon(LifecycleCommand::Restart),
            AppServerHostCommand::Daemon(LifecycleCommand::Stop),
            AppServerHostCommand::Daemon(LifecycleCommand::Version),
        ]
    );
}

#[test]
fn app_server_lifecycle_commands_preserve_explicit_product_services() {
    assert_eq!(
        parse_arguments(&[
            "daemon".into(),
            "start".into(),
            "--product-services".into(),
            "product-services.json".into(),
        ])
        .unwrap(),
        (
            AppServerHostCommand::Daemon(LifecycleCommand::Start),
            Some("product-services.json".into()),
        )
    );
    assert!(parse_arguments(&["daemon".into(), "unknown".into()]).is_err());
}
