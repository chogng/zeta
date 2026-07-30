use super::{BootstrapOutputFilter, SHELL_BOOTSTRAP_MARKER, TerminalSessionEvent, shell_bootstrap};
use zeta_terminal::{GridSize, TerminalCore};

#[test]
fn terminal_events_update_grid_blocks_and_exit_state() {
    let mut core = TerminalCore::new(GridSize::new(3, 16));
    TerminalSessionEvent::Output(b"ready\r\n".to_vec()).apply_to(&mut core);
    core.start_command("exit 3");
    TerminalSessionEvent::Exited(3).apply_to(&mut core);

    assert_eq!(core.grid().lines()[0].text(), "ready");
    assert_eq!(core.exit_code(), Some(3));
    assert_eq!(core.block_list().blocks()[0].command(), "exit 3");
}

#[test]
fn terminal_output_queries_leave_reply_bytes_for_the_session_owner() {
    let mut core = TerminalCore::new(GridSize::new(3, 16));
    TerminalSessionEvent::Output(b"\x1b[2;4H\x1b[6n".to_vec()).apply_to(&mut core);

    assert_eq!(core.take_reply_bytes(), b"\x1b[2;4R");
}

#[test]
fn supported_shell_bootstrap_disables_echo_and_hides_the_native_prompt() {
    let bootstrap = shell_bootstrap("/bin/zsh").unwrap();
    let input = String::from_utf8(bootstrap.input).unwrap();

    assert!(input.contains("stty -echo"));
    assert!(input.contains("PROMPT=$'\\033[0m'"));
    assert!(input.contains("PROMPT_EOL_MARK=''"));
    assert!(input.contains("precmd_functions=(__zeterm_precmd)"));
    assert!(input.contains("]133;D;"));
    assert_eq!(bootstrap.marker, SHELL_BOOTSTRAP_MARKER);
    assert!(shell_bootstrap("/usr/bin/fish").is_none());
}

#[test]
fn bootstrap_output_filter_discards_startup_output_across_chunk_boundaries() {
    let mut filter = BootstrapOutputFilter::new(SHELL_BOOTSTRAP_MARKER);

    assert_eq!(
        filter.push(b"prompt bootstrap\x1b]9;zeterm-".to_vec()),
        None
    );
    assert_eq!(
        filter.push(b"ready\x07command-output".to_vec()),
        Some(b"command-output".to_vec())
    );
}

#[cfg(unix)]
#[test]
fn pty_output_feeds_the_terminal_core() {
    use std::collections::HashMap;

    use zeta_utils_pty::{SpawnedProcess, TerminalSize, spawn_pty_process};

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let cwd = std::env::current_dir().unwrap();
    let environment = std::env::vars().collect::<HashMap<_, _>>();
    let spawned = runtime
        .block_on(spawn_pty_process(
            "/bin/sh",
            &[
                "-c".to_string(),
                "printf '\\033[32mpty-ready\\033[0m\\n'".to_string(),
            ],
            &cwd,
            &environment,
            &None,
            TerminalSize { rows: 4, cols: 32 },
            &[],
        ))
        .unwrap();
    let SpawnedProcess {
        session,
        mut stdout_rx,
        stderr_rx: _,
        exit_rx,
    } = spawned;
    assert_eq!(runtime.block_on(exit_rx).unwrap(), 0);
    session.release_pty_handles_after_exit();
    let output = runtime.block_on(async move {
        let mut output = Vec::new();
        while let Some(chunk) = stdout_rx.recv().await {
            output.extend(chunk);
        }
        output
    });
    let mut core = TerminalCore::new(GridSize::new(4, 32));
    core.process_output(&output);

    assert!(
        core.grid()
            .lines()
            .iter()
            .any(|line| line.text().contains("pty-ready"))
    );
}
