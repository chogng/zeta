use super::Command;
use super::parse;
use crate::LifecycleCommand;

#[test]
fn lifecycle_and_connection_commands_preserve_explicit_product_services() {
    for (argument, expected) in [
        ("connect", Command::Connect),
        ("start", Command::Lifecycle(LifecycleCommand::Start)),
        ("restart", Command::Lifecycle(LifecycleCommand::Restart)),
        ("stop", Command::Lifecycle(LifecycleCommand::Stop)),
        ("version", Command::Lifecycle(LifecycleCommand::Version)),
    ] {
        assert_eq!(
            parse(&[
                argument.into(),
                "--product-services".into(),
                "services.json".into()
            ])
            .unwrap(),
            (expected, Some("services.json".into()))
        );
    }
    for arguments in [
        vec![],
        vec!["unknown"],
        vec!["start", "extra"],
        vec!["start", "--product-services"],
    ] {
        assert!(parse(&arguments.into_iter().map(Into::into).collect::<Vec<_>>()).is_err());
    }
}
