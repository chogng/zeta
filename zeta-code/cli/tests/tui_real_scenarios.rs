#![cfg(unix)]

#[path = "support/scenario_http.rs"]
mod scenario_http;
#[path = "support/tui_process.rs"]
mod tui_process;

use scenario_http::Gate;
use scenario_http::HttpResponse;
use scenario_http::ScenarioServer;
use std::fs;
use std::process::Command;
use tui_process::Fixture;
use tui_process::LARGE_SIZE;
use tui_process::SMALL_SIZE;
use tui_process::TuiProcess;

#[test]
fn actual_tui_runs_three_complete_conversation_turns() {
    let fixture = Fixture::new("multi-turn-trajectory");
    let first = Gate::new();
    let second = Gate::new();
    let third = Gate::new();
    let server = ScenarioServer::start([
        HttpResponse::streaming(
            [
                "第一轮正在流式回答：",
                "Zeta 已收到中文、English 与 emoji 🚀。",
            ],
            Some(first.clone()),
        ),
        HttpResponse::streaming(
            ["第二轮会引用上一轮：", "上下文仍然完整。"],
            Some(second.clone()),
        ),
        HttpResponse::streaming(
            ["第三轮最终结论：", "连续多轮对话已完成。"],
            Some(third.clone()),
        ),
    ]);
    fixture.write_config(&server.base_url());

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.assert_snapshot("real/01-conversation/00-started");

    process.type_text("第一轮：请确认输入、流式输出和 Unicode 🚀");
    process.enter();
    first.wait_until_reached();
    process.wait_for_screen("第一轮正在流式回答");
    process.assert_snapshot("real/01-conversation/01-first-turn-streaming");
    first.release();
    process.wait_for_stable_screen("Zeta 已收到中文、English 与 emoji");

    process.type_text("第二轮：请明确引用上一轮上下文");
    process.enter();
    second.wait_until_reached();
    process.wait_for_screen("第二轮会引用上一轮");
    second.release();
    process.wait_for_stable_screen("上下文仍然完整");

    process.type_text("第三轮：总结前三轮是否稳定");
    process.enter();
    third.wait_until_reached();
    process.wait_for_screen("第三轮最终结论");
    third.release();
    process.wait_for_stable_screen("连续多轮对话已完成");
    process.assert_snapshot("real/01-conversation/02-third-turn-complete");

    let bodies = server.request_bodies();
    assert_eq!(bodies.len(), 3);
    assert!(bodies[1].contains("第一轮正在流式回答"));
    assert!(bodies[2].contains("第二轮会引用上一轮"));
    assert!(bodies[2].contains("第三轮：总结前三轮是否稳定"));
    process.quit();
}

#[test]
fn actual_tui_displays_git_branch_and_changes() {
    let fixture = Fixture::new("git-status");
    let initialized = Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .current_dir(fixture.workspace())
        .status()
        .unwrap();
    assert!(initialized.success());
    fs::write(fixture.workspace().join("changed.txt"), "uncommitted\n").unwrap();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_stable_screen("zeta-real-scenario · main · 1 change");
    process.assert_snapshot("real/08-git/00-branch-and-change");
    process.quit();
}

#[test]
fn actual_tui_navigates_config_tabs_and_temporary_pickers() {
    let fixture = Fixture::new("temporary-pages");
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");

    process.type_text("/");
    process.assert_snapshot("real/02-pages/00-slash-completion-open");
    process.type_text("config");
    process.enter();
    process.wait_for_screen("Mouse interactions");
    process.tab();
    process.tab();
    process.down();
    process.type_text("OpenAI-compatible");
    process.wait_for_screen("OpenAI-compatible");
    process.down();
    process.enter();
    process.wait_for_screen("OpenAI-compatible API key");
    process.type_text("sk-test-not-a-real-secret");
    process.assert_snapshot("real/02-pages/01-provider-key-input-masked");
    process.enter();
    process.wait_for_screen("OpenAI-compatible");
    process.tab();
    process.back_tab();
    process.escape();
    process.wait_for_stable_screen("zeta-real-scenario");

    process.type_text("/statusline");
    process.enter();
    process.wait_for_screen("Git branch");
    process.down();
    process.enter();
    process.wait_for_screen("Git branch");
    process.escape();

    process.submit("/theme");
    process.wait_for_screen("Diff preview");
    process.assert_snapshot("real/02-pages/02-theme-open");
    process.down();
    process.enter();
    process.wait_for_stable_screen("Theme set to");

    process.submit("/help");
    process.wait_for_screen("Search commands");
    process.assert_snapshot("real/02-pages/03-help-open");
    process.down();
    process.escape();
    process.quit();
}

#[test]
fn actual_tui_queues_restores_and_completes_messages() {
    let fixture = Fixture::new("queue-operations");
    let first_gate = Gate::new();
    let server = ScenarioServer::start([
        HttpResponse::streaming(
            ["首轮仍在运行，队列可以编辑。", "首轮现在完成。"],
            Some(first_gate.clone()),
        ),
        HttpResponse::streaming(["队列中保留的消息已经发送。"], None),
    ]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.submit("首轮：保持运行以便操作队列");
    first_gate.wait_until_reached();
    process.wait_for_screen("首轮仍在运行");

    process.type_text("第二条：应该保留并自动发送");
    process.enter();
    process.wait_for_screen("第二条：应该保留并自动发送");
    process.type_text("第三条：稍后恢复到输入框");
    process.enter();
    process.wait_for_screen("第三条：稍后恢复到输入框");
    process.assert_snapshot("real/04-queue/00-messages-queued");

    process.type_text("/queue");
    process.enter();
    process.wait_for_screen("Enter view");
    process.down();
    process.enter();
    process.wait_for_screen("Queued message");
    process.assert_snapshot("real/04-queue/01-message-detail-open");
    process.escape();
    process.wait_for_screen("Enter view");
    process.send(b"r");
    process.wait_for_screen("❯ 第三条：稍后恢复到输入框\n────────────────");
    process.assert_snapshot("real/04-queue/02-message-restored-to-input");

    first_gate.release();
    process.wait_for_stable_screen("队列中保留的消息已经发送");
    process.assert_snapshot("real/04-queue/03-remaining-message-completed");
    assert_eq!(server.request_count(), 2);
    let bodies = server.request_bodies();
    assert!(bodies[1].contains("第二条：应该保留并自动发送"));
    assert!(!bodies[1].contains("第三条：稍后恢复到输入框"));
    process.quit();
}

#[test]
fn actual_tui_recovers_from_auth_and_rate_limit_failures() {
    let auth_fixture = Fixture::new("http-401");
    let auth_server = ScenarioServer::start([
        HttpResponse::failure(401, "invalid-test-credential"),
        HttpResponse::streaming(["401 之后的下一轮恢复成功。"], None),
    ]);
    auth_fixture.write_config(&auth_server.base_url());
    let mut auth = TuiProcess::start(&auth_fixture, &[], LARGE_SIZE);
    auth.wait_for_screen("Tips for getting started");
    auth.type_text("触发 401 鉴权失败");
    auth.enter();
    auth.wait_for_stable_screen("Model provider authentication failed");
    auth.assert_snapshot("real/05-errors/00-auth-failure");
    assert_eq!(auth_server.request_count(), 1);
    auth.type_text("鉴权失败后继续下一轮");
    auth.enter();
    auth.wait_for_stable_screen("401 之后的下一轮恢复成功");
    auth.assert_snapshot("real/05-errors/01-auth-recovered");
    auth.quit();

    let rate_fixture = Fixture::new("http-429");
    let rate_server = ScenarioServer::start([
        HttpResponse::failure(429, "test-rate-limit"),
        HttpResponse::failure(429, "test-rate-limit"),
        HttpResponse::failure(429, "test-rate-limit"),
        HttpResponse::failure(429, "test-rate-limit"),
        HttpResponse::streaming(["429 重试耗尽后仍可恢复。"], None),
    ]);
    rate_fixture.write_config(&rate_server.base_url());
    let mut rate = TuiProcess::start(&rate_fixture, &[], LARGE_SIZE);
    rate.wait_for_screen("Tips for getting started");
    rate.type_text("触发 429 限流和自动重试");
    rate.enter();
    rate.wait_for_stable_screen("Model invocation failed");
    rate.assert_snapshot("real/05-errors/02-rate-limit-retries-exhausted");
    assert_eq!(rate_server.request_count(), 4);
    rate.submit("限流失败后继续下一轮");
    rate.wait_for_stable_screen("429 重试耗尽后仍可恢复");
    rate.assert_snapshot("real/05-errors/03-rate-limit-recovered");
    rate.quit();
}

#[test]
fn actual_tui_navigates_the_agents_session_manager_and_preview() {
    let fixture = Fixture::new("agents-manager");
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.wait_for_screen("← agents");
    process.assert_snapshot("real/06-agents/00-session-with-agents-hint");

    process.left();
    process.wait_for_screen("enter create");
    process.up();
    process.wait_for_screen("space to collapse");
    process.down();
    process.wait_for_screen("space to preview");
    process.space();
    process.wait_for_screen("Session preview");
    process.escape();
    process.wait_for_screen("space to preview");
    process.escape();
    process.wait_for_screen("enter create");
    process.right();
    process.wait_for_screen("← agents");
    process.left();
    process.wait_for_screen("enter create");
    process.up();
    process.wait_for_screen("space to preview");
    process.enter();
    process.wait_for_stable_screen("Resumed session");
    process.wait_for_screen("← agents");
    process.assert_snapshot("real/06-agents/10-enter-resumes-selected-session");
    process.left();
    process.wait_for_screen("enter create");
    process.quit();

    let command_fixture = Fixture::new("agents-command");
    let command_server = ScenarioServer::start([]);
    command_fixture.write_config(&command_server.base_url());
    let mut command = TuiProcess::start(&command_fixture, &[], LARGE_SIZE);
    command.wait_for_screen("← agents");
    command.type_text("/agents");
    command.wait_for_screen("open the Session Manager");
    command.enter();
    command.wait_for_screen("enter create");
    command.escape();
    command.wait_for_screen("← agents");
    command.quit();
}

#[test]
fn actual_tui_approves_and_declines_real_file_tool_calls() {
    let approve_fixture = Fixture::new("approve-tool");
    let approved_path = approve_fixture.workspace().join("approved-by-tui.txt");
    let approve_gate = Gate::new();
    let approve_server = ScenarioServer::start([
        HttpResponse::tool_call(
            "call-approve",
            "write_file",
            serde_json::json!({
                "path": approved_path,
                "content": "approved through the real TUI\n",
            }),
        ),
        HttpResponse::streaming(
            ["工具已获批准并执行。", "文件写入完成。"],
            Some(approve_gate.clone()),
        ),
    ]);
    approve_fixture.write_config(&approve_server.base_url());
    let mut approve = TuiProcess::start(&approve_fixture, &[], LARGE_SIZE);
    approve.wait_for_screen("Tips for getting started");
    approve.type_text("请创建 approved-by-tui.txt");
    approve.enter();
    approve.wait_for_screen("Approval required");
    approve.assert_snapshot("real/03-approval/00-approve-request");
    approve.down();
    approve.up();
    approve.enter();
    approve_gate.wait_until_reached();
    approve.wait_for_screen("工具已获批准并执行");
    approve_gate.release();
    approve.wait_for_stable_screen("文件写入完成");
    approve.assert_snapshot("real/03-approval/01-approved-final");
    let approve_bodies = approve_server.request_bodies();
    assert!(
        approve_bodies[1].contains("wrote"),
        "tool result request: {}",
        approve_bodies[1]
    );
    let approved_thread_path = approve_fixture.find_file("approved-by-tui.txt").unwrap();
    assert_eq!(
        std::fs::read_to_string(approved_thread_path).unwrap(),
        "approved through the real TUI\n"
    );
    approve.quit();

    let decline_fixture = Fixture::new("decline-tool");
    let declined_path = decline_fixture.workspace().join("declined-by-tui.txt");
    let decline_gate = Gate::new();
    let decline_server = ScenarioServer::start([
        HttpResponse::tool_call(
            "call-decline",
            "write_file",
            serde_json::json!({
                "path": declined_path,
                "content": "this must never be written\n",
            }),
        ),
        HttpResponse::streaming(
            ["工具调用被用户拒绝。", "没有写入文件。"],
            Some(decline_gate.clone()),
        ),
    ]);
    decline_fixture.write_config(&decline_server.base_url());
    let mut decline = TuiProcess::start(&decline_fixture, &[], LARGE_SIZE);
    decline.wait_for_screen("Tips for getting started");
    decline.submit("请尝试创建 declined-by-tui.txt");
    decline.wait_for_screen("Approval required");
    decline.down();
    decline.assert_snapshot("real/03-approval/02-decline-selected");
    decline.enter();
    decline_gate.wait_until_reached();
    decline.wait_for_screen("工具调用被用户拒绝");
    decline_gate.release();
    decline.wait_for_stable_screen("没有写入文件");
    decline.assert_snapshot("real/03-approval/03-declined-final");
    assert!(decline_fixture.find_file("declined-by-tui.txt").is_none());
    assert!(decline_server.request_bodies()[1].contains("declin"));
    decline.quit();
}

#[test]
fn actual_tui_process_streams_queues_resizes_and_resumes() {
    let fixture = Fixture::new("stream-queue-resume");
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
    let args = ["--resume", session_id.as_str(), thread_id.as_str()];
    let mut resumed = TuiProcess::start(&fixture, &args, LARGE_SIZE);
    resumed.wait_for_stable_screen("第二轮排队消息已经执行。");
    resumed.assert_snapshot("real/07-lifecycle/01-resumed");
    resumed.quit();
}

#[test]
fn actual_tui_process_interrupts_an_inflight_http_stream() {
    let fixture = Fixture::new("interrupt");
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
    let fixture = Fixture::new("http-failure");
    let server = ScenarioServer::start([
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::failure(500, "real-http-500"),
        HttpResponse::streaming(["错误后仍能继续对话。"], None),
    ]);
    fixture.write_config(&server.base_url());

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Tips for getting started");
    process.submit("触发真实 HTTP 500");
    process.wait_for_stable_screen("Model invocation failed");
    process.assert_snapshot("real/07-lifecycle/03-http-500");
    assert_eq!(server.request_count(), 4);

    process.submit("错误以后继续发送");
    process.wait_for_stable_screen("错误后仍能继续对话。");
    process.assert_snapshot("real/07-lifecycle/04-error-recovered");
    process.quit();
}
