use crate::scenario_http::HttpResponse;
use crate::scenario_http::ScenarioServer;
use crate::tui_process::Fixture;
use crate::tui_process::LARGE_SIZE;
use crate::tui_process::SMALL_SIZE;
use crate::tui_process::TuiProcess;
use std::fs;

fn open_provider(process: &mut TuiProcess, label: &str) {
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Enhanced TUI");
    process.up();
    process.up();
    process.tab();
    process.down();
    process.type_text(label);
    process.down();
    process.enter();
}

#[test]
fn actual_tui_copy_on_select_is_opt_in_and_persists_across_restart() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Copy on select");
    assert!(process.screen().contains("General"));
    assert!(
        process
            .screen()
            .lines()
            .find(|line| line.contains("Copy on select"))
            .unwrap()
            .contains("[   ]")
    );
    process.down();
    process.enter();
    assert!(fixture.config_source().contains("copyOnSelect = true"));
    assert!(fixture.config_source().contains("mouseInteractions = true"));
    process.escape();
    process.quit();

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Copy on select");
    assert!(
        process
            .screen()
            .lines()
            .find(|line| line.contains("Copy on select"))
            .unwrap()
            .contains("[ ✔ ]")
    );
    process.down();
    process.enter();
    assert!(fixture.config_source().contains("copyOnSelect = false"));
    assert!(fixture.config_source().contains("mouseInteractions = true"));
    process.escape();
    process.quit();
}

#[test]
fn actual_tui_issue_config_switch_gates_its_tab() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Recommend issue grouping");
    process.up();
    process.up();
    process.back_tab();
    process.wait_for_screen("Not configured - choose a model");
    process.tab();
    process.enter();
    for _ in 0..5 { process.down(); }
    process.enter();
    assert!(fixture.config_source().contains("recommendMerge = false"));
    process.send(b"\x1b[H");
    process.up();
    process.up();
    process.back_tab();
    process.wait_for_screen("No language servers configured");
    process.tab();
    process.enter();
    for _ in 0..5 { process.down(); }
    process.enter();
    assert!(fixture.config_source().contains("recommendMerge = true"));
    process.send(b"\x1b[H");
    process.up();
    process.up();
    process.back_tab();
    process.wait_for_screen("Not configured - choose a model");
    process.resize(SMALL_SIZE);
    process.wait_for_screen("Analysis model");
    process.escape();
    process.quit();
    assert!(server.request_bodies().is_empty(), "configuring the feature must not invoke a model");
}

#[test]
fn actual_tui_provider_autosaves_and_tests_without_fetching_models() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([
        HttpResponse::Json { body: br#"{"id":"test","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OK"}]}],"status":"completed","usage":{"input_tokens":1,"output_tokens":1}}"#.to_vec() },
        HttpResponse::failure(401, "synthetic-denial"),
    ]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    open_provider(&mut process, "New custom provider");
    process.wait_for_screen("> Provider name");
    for (index, value) in ["PTY service", &format!("{}/", server.base_url()), "pty-synthetic-key"].into_iter().enumerate() {
        process.enter();
        process.type_text(value);
        process.enter();
        if index > 0 { process.wait_for_screen("Saved"); }
        process.down();
    }
    assert!(fixture.config_source().contains("PTY service"));
    process.wait_for_screen("> Model ID");
    process.enter();
    process.type_text("pty-model-alias");
    process.enter();
    process.wait_for_screen("Saved");
    process.down(); // API type.
    process.down(); // Model context window.
    process.wait_for_screen("Model context window");
    process.right();
    process.wait_for_screen("1m");
    process.wait_for_screen("Saved");
    assert!(fixture.config_source().contains("1000000"));
    process.down();
    process.enter();
    process.wait_for_screen("Passed");
    process.enter();
    process.wait_for_screen("Authentication failed");
    assert!(fixture.config_source().contains("pty-model-alias"));
    assert!(!fixture.config_source().contains("pty-synthetic-key"));
    process.escape();
    process.wait_for_screen("> PTY service");
    process.escape();
    process.submit("/model");
    process.wait_for_screen("Favorites");
    process.tab();
    process.down();
    process.wait_for_screen("pty-model-alias");
    process.type_text("p");
    process.wait_for_screen("Model pinned");
    assert!(fixture.config_source().contains("pinnedModels"));
    process.escape();
    process.submit("/model");
    process.wait_for_screen("> pty-model-alias");
    process.tab(); // Switch provider tab and focus its header.
    process.enter(); // Enter its list, without applying the selected model.
    process.wait_for_screen("> pty-model-alias");
    process.escape();
    open_provider(&mut process, "PTY service");
    process.wait_for_screen("> Provider name");
    process.enter();
    process.type_text("-cancelled");
    process.escape();
    assert!(!process.screen().contains("-cancelled"));
    process.down();
    process.down();
    process.wait_for_screen("Key saved");
    process.resize(SMALL_SIZE);
    for _ in 0..4 { process.down(); }
    process.wait_for_screen("Test");
    process.escape();
    process.send(b"\x1b[3~");
    process.wait_for_screen("No matching configuration");
    assert!(!fixture.config_source().contains("PTY service"));
    assert!(!fixture.config_source().contains("pty-model-alias"));
    process.escape();
    process.quit();
    let requests = server.request_bodies();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert!(request.starts_with("POST /v1/responses "));
        assert!(request.contains("pty-model-alias"));
        assert!(request.contains("Bearer pty-synthetic-key"));
    }
}

#[test]
fn actual_tui_opens_chatgpt_subscription_and_returns_to_providers() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    open_provider(&mut process, "ChatGPT");
    process.wait_for_screen("Not signed in");
    process.wait_for_screen("Sign in with ChatGPT");
    process.resize(SMALL_SIZE);
    process.wait_for_screen("Sign in with ChatGPT");
    process.escape();
    process.wait_for_screen("Providers");
    process.escape();
    process.quit();
    assert!(server.request_bodies().is_empty());
}

#[test]
fn actual_tui_reuses_chatgpt_subscription_without_changing_codex_auth() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    fs::create_dir_all(fixture.codex_home()).unwrap();
    // Synthetic JWTs identify account@example.invalid / pro and expire in 2096.
    // This test only reads accounts and reconnects; it never invokes a model.
    let original = serde_json::to_vec(&serde_json::json!({
        "auth_mode": "chatgpt", "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": "e30.eyJlbWFpbCI6ImFjY291bnRAZXhhbXBsZS5pbnZhbGlkIiwiaHR0cHM6Ly9hcGkub3BlbmFpLmNvbS9hdXRoIjp7ImNoYXRncHRfYWNjb3VudF9pZCI6ImFjY291bnQtMSIsImNoYXRncHRfcGxhbl90eXBlIjoicHJvIn19.signature",
            "access_token": "e30.eyJleHAiOjQwMDAwMDAwMDB9.signature",
            "refresh_token": "test-refresh-never-used", "account_id": "account-1"
        },
        "last_refresh": "2026-09-07T00:00:00Z"
    })).unwrap();
    let auth = fixture.codex_home().join("auth.json");
    fs::write(&auth, &original).unwrap();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    open_provider(&mut process, "ChatGPT");
    process.wait_for_screen("account@example.invalid");
    process.wait_for_screen("Disconnect from Zeta");
    process.down();
    process.down();
    process.down();
    process.enter();
    process.wait_for_screen("Disconnected from ChatGPT in Zeta");
    assert!(fs::read(&auth).unwrap() == original);
    process.send(b"\x1b[H");
    process.down();
    process.down();
    process.enter();
    process.wait_for_screen("account@example.invalid");
    process.escape();
    process.escape();
    process.quit();
    assert!(fs::read(auth).unwrap() == original);
    assert!(server.request_bodies().is_empty());
}

#[test]
fn actual_tui_switches_language_and_persists_it() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Enhanced TUI");
    process.down();
    process.down();
    process.down();
    process.down();
    process.down();
    process.enter();
    process.wait_for_screen("拡張 TUI");
    process.escape();
    process.quit();

    assert!(fixture.config_source().contains("language = \"ja\""));
}

#[test]
fn actual_tui_navigates_config_tabs_and_temporary_pickers() {
    let fixture = Fixture::new();
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
    process.type_text("OpenAI");
    process.wait_for_screen("OpenAI");
    process.down();
    process.enter();
    process.wait_for_screen("Base URL (read-only)");
    process.type_text("sk-test-not-a-real-secret");
    process.wait_for_screen("••••");
    process.enter();
    process.wait_for_screen("Saved · Connection not verified");
    process.escape();
    process.escape();
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
fn actual_tui_config_enables_and_disables_memory_diagnostics() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Memory diagnostics");
    process.down();
    process.down();
    process.enter();
    wait_for_config(&fixture, "memoryDiagnostics = true");
    process.escape();

    process.submit("/status");
    process.wait_for_screen("Thread");
    process.tab();
    process.wait_for_screen("Memory diagnostics");
    process.wait_for_stable_screen("Recording");
    process.escape();

    process.submit("/config");
    process.wait_for_screen("Memory diagnostics");
    process.down();
    process.down();
    process.enter();
    wait_for_config(&fixture, "memoryDiagnostics = false");
    process.escape();
    process.submit("/status");
    process.wait_for_screen("Thread");
    process.tab();
    process.wait_for_screen("Memory diagnostics");
    process.wait_for_stable_screen("Disabled");
    process.quit();
    assert!(
        server.request_bodies().is_empty(),
        "diagnostics must not invoke a model"
    );
}

#[test]
fn actual_tui_tab_switches_from_content_search_and_unsaved_field() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Enhanced TUI");
    let original_config = fixture.config_source();
    process.tab();
    process.wait_for_screen("> OpenAI");
    process.type_text("/OpenAI");
    process.tab();
    process.wait_for_screen("No matching configuration");
    assert!(process.screen().contains("OpenAI"));
    process.back_tab();
    process.enter();
    process.wait_for_screen("> OpenAI");
    process.enter();
    process.wait_for_screen("> API key");
    process.back_tab();
    process.wait_for_screen("> Provider name");
    process.enter();
    process.type_text("Unsubmitted service");
    process.tab();
    process.wait_for_screen("> API key");
    process.back_tab();
    process.wait_for_screen("Unsubmitted service");
    process.type_text(" continued");
    process.wait_for_screen("Unsubmitted service continued");
    process.resize(SMALL_SIZE);
    process.wait_for_screen("Unsubmitted service continued");
    process.escape();
    assert!(!process.screen().contains("Unsubmitted service"));
    assert_eq!(fixture.config_source(), original_config);
    process.escape();
    process.escape();
    process.escape();
    process.quit();

    assert!(server.request_bodies().is_empty());
}

#[test]
fn actual_tui_status_line_style_persists_across_restart() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Enhanced TUI");
    for _ in 0..7 {
        process.down();
    }
    process.enter();
    process.wait_for_screen("Emoji and progress bars at a glance");
    process.escape();
    process.wait_for_screen("🤖 zeta-real-scenario");
    process.quit();
    assert!(
        fixture
            .config_source()
            .contains("statusLineStyle = \"rich\"")
    );

    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("🤖 zeta-real-scenario");
    process.submit("/config");
    process.wait_for_screen("Emoji and progress bars at a glance");
    for _ in 0..7 {
        process.down();
    }
    process.enter();
    process.wait_for_screen("Text and numbers, clean and easy to read");
    process.escape();
    process.quit();
    assert!(
        fixture
            .config_source()
            .contains("statusLineStyle = \"compact\"")
    );
    assert!(server.request_bodies().is_empty());
}

fn wait_for_config(fixture: &Fixture, expected: &str) {
    let path = fixture
        .find_file("config.toml")
        .expect("the scenario config exists");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let config = fs::read_to_string(&path).unwrap_or_default();
        if config.contains(expected) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "config did not contain {expected:?}:\n{config}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
