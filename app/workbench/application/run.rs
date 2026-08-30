use super::*;

pub fn run() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == zeta_app_server_daemon::DAEMON_PROCESS_ARGUMENT)
    {
        return match zeta_app_server_daemon::run_from_environment(arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("app App Server daemon: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if arguments
        .first()
        .is_some_and(|command| command == "app-server")
    {
        return match zeta_server_host::run_app_server(arguments.into_iter().skip(1)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("app App Server host: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let invocation = match AppInvocation::parse(arguments) {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("{error}");
            return if error.is_help_requested() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
    };
    let mut launch = match invocation.resolve() {
        Ok(Some(launch)) => launch,
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = launch_progress::prepare_remote_launch(&mut launch) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let application_exit =
        match Application::run(move |event_proxy| WorkbenchApplication::new(event_proxy, launch)) {
            Ok(application_exit) => application_exit,
            Err(error) => {
                eprintln!("failed to run the desktop event loop: {error}");
                return ExitCode::FAILURE;
            }
        };
    let (application, runtime_error, _) = application_exit.into_parts();
    if let Some(error) = runtime_error.as_ref() {
        eprintln!("{APP_DISPLAY_NAME} runtime failed: {error}");
    }
    if application.failed || runtime_error.is_some() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
