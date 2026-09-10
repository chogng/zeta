use crate::scenario_http::Gate;
use crate::scenario_http::HttpResponse;
use crate::scenario_http::ScenarioServer;
use crate::tui_process::Fixture;
use crate::tui_process::LARGE_SIZE;
use crate::tui_process::SMALL_SIZE;
use crate::tui_process::TuiProcess;

#[test]
fn actual_tui_native_preserves_history_across_panels_resize_and_exit() {
    let fixture = Fixture::new();
    let replies = [
        "NATIVE-REPLY-ONE",
        "NATIVE-REPLY-TWO",
        "NATIVE-REPLY-THREE 中文",
    ];
    let server = ScenarioServer::start(replies.map(|reply| HttpResponse::streaming([reply], None)));
    fixture.write_config(&server.base_url());
    fixture.append_config("\n[tui]\nscreenMode = \"native\"\n");
    let mut process = TuiProcess::start_in_vscode(&fixture, &[], LARGE_SIZE);
    process.wait_for_stable_screen("ask permissions on");
    for (index, reply) in replies.iter().enumerate() {
        process.submit(&format!("NATIVE-MESSAGE-{index}"));
        process.wait_for_screen(reply);
        process.wait_for_stable_screen("ask permissions on");
        process.submit("/status");
        process.wait_for_stable_screen("Full context window");
        process.escape();
        process.wait_for_stable_screen("ask permissions on");
        assert_input_surface_visible(&process);
    }
    process.resize(SMALL_SIZE);
    process.wait_for_stable_screen("ask permissions on");
    process.type_text("DRAFT-AFTER-RESIZE");
    process.wait_for_stable_screen("DRAFT-AFTER-RESIZE");
    assert!(!process.raw_text().contains("\x1b[?1049h"));
    assert!(!process.raw_text().contains("\x1b[?1000h"));
    assert!(!process.raw_text().contains("\x1b[?1003h"));
    process.quit();
    let history = process.terminal_text();
    for reply in replies {
        assert_eq!(history.matches(reply).count(), 1, "{history}");
    }
    assert!(
        !history.contains("Full context window"),
        "temporary panels must not enter history:\n{history}"
    );
    assert!(process.raw_text().contains("\x1b[?2004l"));
    assert!(!process.raw_text().contains("\x1b[?1049l"));
    assert_eq!(server.request_count(), replies.len());
}

#[test]
fn actual_tui_screen_mode_switches_live_and_persists() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([HttpResponse::streaming(["MODE-SWITCH-REPLY"], None)]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start_in_vscode(&fixture, &[], LARGE_SIZE);
    process.wait_for_stable_screen("ask permissions on");
    let fullscreen_input = input_top_row(&process);
    // ConPTY can implement the screen switch through console APIs instead of forwarding CSI.
    #[cfg(unix)]
    assert!(process.raw_text().contains("\x1b[?1049h"));
    process.submit("/config");
    process.wait_for_stable_screen("Screen mode");
    for _ in 0..6 {
        process.down();
    }
    process.enter();
    process.wait_for_stable_screen("native");
    assert!(fixture.config_source().contains("screenMode = \"native\""));
    #[cfg(unix)]
    assert!(process.raw_text().contains("\x1b[?1049l"));
    process.escape();
    process.wait_for_stable_screen("ask permissions on");
    assert!(
        input_top_row(&process) < fullscreen_input,
        "{}",
        process.screen()
    );
    process.submit("/config");
    process.wait_for_stable_screen("Screen mode");
    for _ in 0..6 {
        process.down();
    }
    process.enter();
    process.wait_for_stable_screen("fullscreen");
    assert!(
        fixture
            .config_source()
            .contains("screenMode = \"fullscreen\"")
    );
    process.escape();
    process.wait_for_stable_screen("ask permissions on");
    assert_eq!(input_top_row(&process), fullscreen_input);
    process.submit("check the preserved conversation");
    process.wait_for_screen("MODE-SWITCH-REPLY");
    process.quit();
    assert_eq!(server.request_count(), 1);
}

#[test]
fn actual_tui_multiple_commands_preserve_internal_history_and_fixed_input() {
    const REPLIES: [&str; 12] = [
        "REPLY-00", "REPLY-01", "REPLY-02", "REPLY-03", "REPLY-04", "REPLY-05", "REPLY-06",
        "REPLY-07", "REPLY-08", "REPLY-09", "REPLY-10", "REPLY-11",
    ];
    for size in [LARGE_SIZE, SMALL_SIZE] {
        let fixture = Fixture::new();
        let server =
            ScenarioServer::start(REPLIES.map(|reply| HttpResponse::streaming([reply], None)));
        fixture.write_config(&server.base_url());
        let mut process = TuiProcess::start_in_vscode(&fixture, &[], size);
        process.wait_for_stable_screen("ask permissions on");
        for _ in 0..12 {
            process.submit("/status");
            process.wait_for_screen("Full context window");
            process.escape();
            process.wait_for_stable_screen("ask permissions on");
        }
        for (index, reply) in REPLIES.iter().enumerate() {
            process.submit(&format!("MESSAGE-{index:02}"));
            process.wait_for_screen(reply);
            process.wait_for_stable_screen("ask permissions on");
            assert_input_surface_visible(&process);
        }
        assert_eq!(server.request_count(), REPLIES.len());
        process.scroll_up(2, 2);
        process.wait_for_screen("Jump to bottom (click) ↓");
        process.control_home();
        process.wait_for_screen("Zeta Code v");
        let visible_history = process.screen();
        assert!(
            visible_history.contains("> /status"),
            "local commands must remain reachable at the start of the transcript:\n{visible_history}"
        );
        process.control_end();
        process.wait_for_screen("REPLY-11");
        assert!(process.screen().contains("MESSAGE-11"));
        process.quit();
    }
}

#[test]
fn actual_tui_input_keeps_hint_bar_without_blank_line_growth() {
    for size in [LARGE_SIZE, SMALL_SIZE] {
        let fixture = Fixture::new();
        let server = ScenarioServer::start([HttpResponse::streaming(["ISSUE13-REPLY"], None)]);
        fixture.write_config(&server.base_url());
        let mut process = TuiProcess::start_in_vscode(&fixture, &[], size);
        process.wait_for_stable_screen("ask permissions on");
        assert_input_surface_visible(&process);
        let hint_row = |process: &TuiProcess| {
            process
                .screen()
                .lines()
                .position(|line| line.contains("ask permissions on"))
                .expect("hint bar remains visible")
        };
        let initial_hint = hint_row(&process);
        let initial_input = input_top_row(&process);
        for ch in "hello".chars() {
            process.type_text(&ch.to_string());
            assert_eq!(
                hint_row(&process),
                initial_hint,
                "typing must not move the hint bar"
            );
        }
        process.enter();
        process.wait_for_screen("ISSUE13-REPLY");
        process.wait_for_stable_screen("ask permissions on");
        assert_eq!(input_top_row(&process), initial_input);
        assert_eq!(server.request_count(), 1);
        if size.cols == LARGE_SIZE.cols && size.rows == LARGE_SIZE.rows {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let screen = process.screen();
                if !screen.contains("image in clipboard")
                    && !screen.contains("shift+tab to cycle policy")
                {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "temporary input tips did not expire"
                );
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            process.wait_for_stable_screen("ask permissions on");
            process.assert_snapshot("issue13/fullscreen_conversation");
        }
        process.submit("/status");
        process.wait_for_screen("Full context window");
        process.escape();
        process.wait_for_stable_screen("ask permissions on");
        assert_input_surface_visible(&process);
        assert_eq!(input_top_row(&process), initial_input);
        let closed_hint = hint_row(&process);
        process.type_text("next");
        assert_eq!(hint_row(&process), closed_hint);
        process.quit();
    }
}

fn assert_input_surface_visible(process: &TuiProcess) {
    let screen = process.screen();
    assert!(
        screen.lines().filter(|line| line.starts_with("──")).count() >= 2,
        "chat input borders must remain visible:\n{screen}"
    );
}

fn input_top_row(process: &TuiProcess) -> usize {
    process
        .screen()
        .lines()
        .position(|line| line.starts_with("──"))
        .expect("chat input top border remains visible")
}

#[test]
fn actual_tui_pty_streams_utf8_resizes_exits_and_resumes() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([HttpResponse::streaming(["PTY lifecycle reply"], None)]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.resize(LARGE_SIZE);
    process.submit("PTY 中文输入 🚀");
    process.wait_for_screen("PTY lifecycle reply");
    assert_eq!(server.request_count(), 1);
    assert!(server.request_bodies()[0].contains("PTY 中文输入 🚀"));
    process.resize(SMALL_SIZE);
    process.wait_for_screen("PTY lifecycle reply");
    eprintln!("PTY lifecycle: closing first process");
    process.quit();
    eprintln!("PTY lifecycle: reading persisted session");
    let (session, thread) = fixture.only_thread();
    eprintln!("PTY lifecycle: starting resumed process");
    let mut resumed = TuiProcess::start(&fixture, &["resume", &session, &thread], LARGE_SIZE);
    resumed.wait_for_screen("PTY lifecycle reply");
    assert_eq!(
        server.request_count(),
        1,
        "resuming must not invoke a model again"
    );
    eprintln!("PTY lifecycle: closing resumed process");
    resumed.quit();
    eprintln!("PTY lifecycle: complete");
}

#[test]
fn actual_tui_scrolls_the_transcript_with_the_mouse_wheel() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([HttpResponse::streaming(
        [
            "line 01\nline 02\nline 03\nline 04\nline 05\nline 06\nline 07\nline 08\nline 09\nline 10\nline 11\nline 12\nline 13\nline 14\nline 15\nline 16\nline 17\nline 18\nline 19\nline 20",
        ],
        None,
    )]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], SMALL_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("fill the transcript");
    process.wait_for_stable_screen("line 20");

    for _ in 0..5 {
        process.scroll_up(1, 1);
    }

    process.wait_for_stable_screen("Zeta Code v");
    process.wait_for_screen("Jump to bottom (click) ↓");
    let screen = process.screen();
    let (row, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("Jump to bottom (click) ↓"))
        .unwrap();
    let column = line[..line.find("Jump to bottom").unwrap()].chars().count();
    process.send(
        format!(
            "\x1b[<0;{};{}M\x1b[<0;{};{}m",
            column + 1,
            row + 1,
            column + 1,
            row + 1
        )
        .as_bytes(),
    );
    process.wait_for_stable_screen("line 20");
    assert!(!process.screen().contains("Jump to bottom"));
    process.quit();
}

#[cfg(unix)]
#[test]
fn actual_tui_process_details_show_sandbox_enforcement() {
    let fixture = Fixture::new();
    let outside_path = fixture
        .workspace()
        .parent()
        .unwrap()
        .join("sandbox-must-not-write.txt");
    let server = ScenarioServer::start([
        HttpResponse::reasoning_tool_call(
            "先在受限进程中尝试写入工作区外部。\n再根据进程结果确认目录边界是否生效。",
            "call-sandbox",
            "shell-command",
            serde_json::json!({
                "program": "/bin/sh",
                "arguments": [
                    "-c",
                    "touch ../sandbox-must-not-write.txt 2>&- || { echo 'sandbox fixture: operation not permitted' >&2; exit 1; }",
                ],
                "working_directory": ".",
            }),
        ),
        HttpResponse::streaming(["沙盒拒绝了越界写入，目标文件没有生成。"], None),
    ]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.back_tab();
    process.back_tab();
    process.wait_for_screen("bypass permissions on");
    process.submit("尝试在工作区外创建 sandbox-must-not-write.txt");
    process.wait_for_stable_screen("目标文件没有生成");
    process.assert_snapshot("real/03-approval/09-sandbox-blocked");
    assert!(!outside_path.exists());
    assert_eq!(server.request_count(), 2);
    assert!(server.request_bodies()[1].contains("sandbox"));

    process.control_up();
    process.up();
    process.space();
    process.wait_for_screen("shell-command [call-sandbox]");
    process.assert_snapshot("real/03-approval/10-sandbox-process-details");

    process.enter();
    process.wait_for_screen("Transcript cell");
    process.assert_snapshot("real/03-approval/11-sandbox-process-full-details");
    process.escape();

    process.space();
    process.up();
    process.space();
    process.wait_for_screen("再根据进程结果确认目录边界是否生效");
    process.assert_snapshot("real/03-approval/12-reasoning-details");
    process.quit();
}

#[test]
fn actual_tui_process_streams_queues_resizes_and_resumes() {
    let fixture = Fixture::new();
    let first_gate = Gate::new();
    let server = ScenarioServer::start([
        HttpResponse::streaming(
            [
                "真实 TCP 流式第一段 🌊",
                "，随后完成长文本与代码 fn main() {}",
            ],
            Some(first_gate.clone()),
        ),
        HttpResponse::streaming(["第二轮排队消息已经执行。"], None),
    ]);
    fixture.write_config(&server.base_url());

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.submit("第一轮：测试真实流式输出、中文和 Emoji");
    first_gate.wait_until_reached();
    process.wait_for_screen("真实 TCP 流式第一段");

    process.submit("第二轮：在第一轮运行时排队");
    process.wait_for_screen("第二轮：在第一轮运行时排队");
    first_gate.release();

    process.wait_for_stable_screen("第二轮排队消息已经执行。");
    assert_eq!(server.request_count(), 2);
    let bodies = server.request_bodies();
    assert!(bodies[0].contains("第一轮：测试真实流式输出、中文和 Emoji"));
    assert!(bodies[1].contains("真实 TCP 流式第一段"));
    assert!(bodies[1].contains("第二轮：在第一轮运行时排队"));

    process.resize(SMALL_SIZE);
    process.wait_for_stable_screen("第二轮排队消息已经执行。");
    process.assert_snapshot("real/07-lifecycle/00-resized");
    process.quit();

    let (session_id, thread_id) = fixture.only_thread();
    let args = ["resume", session_id.as_str(), thread_id.as_str()];
    let mut resumed = TuiProcess::start(&fixture, &args, LARGE_SIZE);
    resumed.wait_for_stable_screen("第二轮排队消息已经执行。");
    resumed.assert_snapshot("real/07-lifecycle/01-resumed");
    resumed.quit();
}

#[test]
fn actual_tui_process_interrupts_an_inflight_http_stream() {
    let fixture = Fixture::new();
    let gate = Gate::new();
    let server = ScenarioServer::start([HttpResponse::streaming(
        ["这段回复正在等待取消", "不应成为完整回复"],
        Some(gate.clone()),
    )]);
    fixture.write_config(&server.base_url());

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.submit("请保持流式输出，直到我取消");
    gate.wait_until_reached();
    process.wait_for_screen("这段回复正在等待取消");
    process.send(&[0x03]);
    process.wait_for_output("turn interrupted");
    process.assert_snapshot_containing("real/07-lifecycle/02-interrupted", "turn interrupted");
    gate.release();
    process.quit();
}

#[test]
fn actual_tui_process_renders_an_http_failure_and_remains_usable() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::streaming(["错误后仍能继续对话。"], None),
    ]);
    fixture.write_config(&server.base_url());

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("ask permissions on");
    process.submit("触发真实 HTTP 500");
    process.wait_for_stable_screen("Provider request failed (500). Try again later.");
    process.assert_snapshot("real/07-lifecycle/03-http-500");
    assert_eq!(server.request_count(), 4);

    process.submit("错误以后继续发送");
    process.wait_for_stable_screen("错误后仍能继续对话。");
    process.assert_snapshot("real/07-lifecycle/04-error-recovered");
    process.quit();
}

#[test]
fn actual_tui_markdown_links_survive_terminal_output_and_resize() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([HttpResponse::streaming(
        [
            "# LINK-RESULT\n\n",
            "[Linked documentation](https://example.com/zeta-terminal-link)\n\n",
            "| Item | State |\n| --- | --- |\n| LINK-CHECK | complete |",
        ],
        None,
    )]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start_in_vscode(&fixture, &[], LARGE_SIZE);
    process.wait_for_stable_screen("ask permissions on");
    process.submit("show links");
    process.wait_for_screen("LINK-CHECK");
    process.wait_for_stable_screen("ask permissions on");
    // ConPTY supplies its own OSC 8 id parameter; assert the destination, not its generated id.
    let raw = process.raw_text();
    let destinations = raw
        .split("\x1b]8;")
        .skip(1)
        .filter_map(|sequence| {
            let payload = sequence.split(['\x1b', '\x07']).next()?;
            payload.split_once(';').map(|(_, destination)| destination)
        })
        .collect::<Vec<_>>();
    assert!(destinations.contains(&"https://example.com/zeta-terminal-link"));
    assert!(destinations.contains(&""), "terminal link must be closed");
    assert!(process.screen().contains("Linked documentation"));
    assert!(!process.screen().contains("]8;;"));
    process.resize(SMALL_SIZE);
    process.wait_for_stable_screen("LINK-CHECK");
    assert!(process.screen().contains("Linked documentation"));
    assert_eq!(server.request_count(), 1);
    process.quit();
}

#[test]
fn actual_tui_streaming_queue_drains_while_provider_waits_and_input_continues() {
    let fixture = Fixture::new();
    let gate = Gate::new();
    let server = ScenarioServer::start([HttpResponse::streaming(
        [
            "COMMIT-ONE\nCOMMIT-TWO\nCOMMIT-THREE\nCOMMIT-FOUR\nCOMMIT-LAST",
            "\nCOMMIT-DONE",
        ],
        Some(gate.clone()),
    )]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start_in_vscode(&fixture, &[], LARGE_SIZE);
    process.wait_for_stable_screen("ask permissions on");
    process.submit("show the queued lines");
    gate.wait_until_reached();
    process.wait_for_screen("COMMIT-ONE");
    process.type_text("draft stays responsive");
    process.wait_for_screen("COMMIT-LAST");
    assert!(process.screen().contains("draft stays responsive"));
    process.resize(SMALL_SIZE);
    // The provider is still running, so its activity indicator continues to animate.
    process.wait_for_screen("COMMIT-LAST");
    gate.release();
    process.wait_for_stable_screen("COMMIT-DONE");
    assert!(process.screen().contains("draft stays responsive"));
    assert_eq!(server.request_count(), 1);
    process.quit();
}
