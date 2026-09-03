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
    fixture.append_config(
        r#"
[[tui.keybindings]]
key = "ctrl+y"
command = "zetaCode.action.copyLastResponse"
"#,
    );
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
    process.wait_for_stable_screen("Cycle approval mode");
    process.assert_snapshot("real/02-pages/03-help-open");
    process.tab();
    process.wait_for_stable_screen("/status");
    process.assert_snapshot("real/02-pages/04-help-commands");
    process.tab();
    process.wait_for_stable_screen("/compact");
    process.assert_snapshot("real/02-pages/05-help-custom-commands");
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
    let approve_gate = Gate::new();
    let approve_server = ScenarioServer::start([
        HttpResponse::tool_call(
            "call-approve",
            "write_file",
            serde_json::json!({
                "path": "approved-by-tui.txt",
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
    approve.control_up();
    approve.up();
    approve.space();
    approve.wait_for_screen("approved through the real TUI");
    approve.assert_snapshot("real/03-approval/04-ask-permissions-details");
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
    let decline_gate = Gate::new();
    let decline_server = ScenarioServer::start([
        HttpResponse::tool_call(
            "call-decline",
            "write_file",
            serde_json::json!({
                "path": "declined-by-tui.txt",
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
fn actual_tui_approval_modes_change_file_tool_authority() {
    let auto_fixture = Fixture::new("auto-review-tool");
    let auto_gate = Gate::new();
    let auto_review = serde_json::json!({
        "recommendation": "deny",
        "reason": "the fixture automatic reviewer denied this file mutation",
    });
    let auto_server = ScenarioServer::start([
        HttpResponse::tool_call(
            "call-auto-review",
            "write_file",
            serde_json::json!({
                "path": "auto-reviewed.txt",
                "content": "approved by automatic review\n",
            }),
        ),
        HttpResponse::completion(auto_review.to_string()),
        HttpResponse::streaming(
            ["自动审查拒绝了工具。", "文件没有写入。"],
            Some(auto_gate.clone()),
        ),
    ]);
    auto_fixture.write_config(&auto_server.base_url());
    let mut auto = TuiProcess::start(&auto_fixture, &[], LARGE_SIZE);
    auto.wait_for_screen("Tips for getting started");
    auto.back_tab();
    auto.wait_for_screen("auto review on");
    auto.submit("请通过自动审查创建 auto-reviewed.txt");
    auto.wait_for_screen("自动审查拒绝了工具");
    auto_gate.wait_until_reached();
    assert!(auto_fixture.find_file("auto-reviewed.txt").is_none());
    auto.back_tab();
    auto.wait_for_screen("current: auto review on");
    auto.assert_snapshot("real/03-approval/05-auto-review-running");
    auto.control_up();
    auto.up();
    auto.space();
    auto.wait_for_screen("fixture automatic reviewer denied");
    auto.assert_snapshot("real/03-approval/06-auto-review-details");
    auto_gate.release();
    auto.wait_for_stable_screen("文件没有写入");
    let auto_bodies = auto_server.request_bodies();
    assert_eq!(auto_bodies.len(), 3);
    assert!(auto_bodies[1].contains("Return JSON matching this response schema"));
    assert!(auto_bodies[2].contains(r#"zeta_action_policy_feedback:{\"kind\":\"denied\""#));
    auto.quit();

    let bypass_fixture = Fixture::new("bypass-tool");
    let bypass_server = ScenarioServer::start([
        HttpResponse::tool_call(
            "call-bypass",
            "write_file",
            serde_json::json!({
                "path": "permission-bypassed.txt",
                "content": "written with permission bypass\n",
            }),
        ),
        HttpResponse::streaming(["权限确认已绕过。", "文件直接写入完成。"], None),
    ]);
    bypass_fixture.write_config(&bypass_server.base_url());
    let mut bypass = TuiProcess::start(&bypass_fixture, &[], LARGE_SIZE);
    bypass.wait_for_screen("Tips for getting started");
    bypass.back_tab();
    bypass.back_tab();
    bypass.wait_for_screen("bypass permissions on");
    bypass.submit("请直接创建 permission-bypassed.txt");
    bypass.wait_for_stable_screen("文件直接写入完成");
    bypass.assert_snapshot("real/03-approval/07-bypass-final");
    bypass.control_up();
    bypass.up();
    bypass.space();
    bypass.wait_for_screen("written with permission bypass");
    bypass.assert_snapshot("real/03-approval/08-bypass-details");
    let bypassed_thread_path = bypass_fixture.find_file("permission-bypassed.txt").unwrap();
    assert_eq!(
        fs::read_to_string(bypassed_thread_path).unwrap(),
        "written with permission bypass\n"
    );
    assert_eq!(bypass_server.request_count(), 2);
    assert!(bypass_server.request_bodies()[1].contains("wrote"));
    bypass.quit();
}

#[test]
fn actual_tui_process_details_show_sandbox_enforcement() {
    let fixture = Fixture::new("sandbox-process");
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
