use crate::scenario_http::HttpResponse;
use crate::scenario_http::ScenarioServer;
use crate::tui_process::Fixture;
use crate::tui_process::LARGE_SIZE;
use crate::tui_process::SMALL_SIZE;
use crate::tui_process::TuiProcess;
use std::fs;
use std::process::Command;

#[test]
fn actual_tui_issue_config_refresh_remains_available_when_grouping_is_disabled() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/config");
    process.wait_for_screen("Recommend issue grouping");
    for _ in 0..5 {
        process.down();
    }
    process.enter();
    wait_for_config(&fixture, "recommendMerge = false");
    process.send(b"\x1b[H");
    process.up();
    process.up();
    process.back_tab();
    process.wait_for_screen("Auto refresh");
    process.enter();
    // Move past the disabled model row to Never.
    process.down();
    process.enter();
    wait_for_config(&fixture, "autoRefreshMinutes = 0");
    assert!(fixture.config_source().contains("recommendMerge = false"));
    process.resize(SMALL_SIZE);
    process.wait_for_stable_screen("Auto refresh");
    process.assert_snapshot("issues/refresh_without_recommendations");
    process.escape();
    process.quit();
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

#[test]
#[cfg(unix)]
fn actual_tui_issue_selection_creates_one_draft_without_touching_source_changes() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    fixture.install_issue_provider();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["config", "user.name", "Issue Test"],
        vec!["config", "user.email", "issue@example.invalid"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["config", "core.hooksPath", "/dev/null"],
        vec![
            "remote",
            "add",
            "origin",
            "https://github.com/team/repo.git",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(fixture.workspace())
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(fixture.workspace().join("tracked.txt"), "base").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(fixture.workspace())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-c", "commit.gpgsign=false", "commit", "-m", "base"])
            .current_dir(fixture.workspace())
            .status()
            .unwrap()
            .success()
    );
    fs::write(fixture.workspace().join("tracked.txt"), "keep my changes").unwrap();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("\u{2190} for agents");
    process.right();
    process.wait_for_screen("Repair first issue");
    process.wait_for_screen("Closed");
    assert!(process.screen().contains("─ Issues"));
    process.send(b"\x1b[F");
    process.right();
    process.wait_for_screen("Previously resolved issue");
    assert!(process.screen().contains("Repair first issue"));
    process.resize(SMALL_SIZE);
    process.wait_for_screen("Closed (1)");
    process.resize(LARGE_SIZE);
    process.wait_for_stable_screen(&format!("─ Issues {}", "─".repeat(80)));
    process.left();
    process.wait_for_stable_screen("▸ Closed (1)");
    assert!(!process.screen().contains("Previously resolved issue"));
    process.send(b"\x1b[H");
    process.down();
    process.space();
    process.down();
    process.space();
    process.wait_for_screen("2 selected");
    process.resize(SMALL_SIZE);
    process.wait_for_screen("2 selected");
    process.resize(LARGE_SIZE);
    process.type_text("o");
    process.wait_for_screen("[issue #3] [issue #5]");
    process.type_text("implement together");
    process.wait_for_screen("implement together");
    assert_eq!(
        fs::read_to_string(fixture.workspace().join("tracked.txt")).unwrap(),
        "keep my changes"
    );
    process.quit();
    assert!(
        server.request_bodies().is_empty(),
        "creating the draft must not invoke the model"
    );
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_auto_squash_retry_preserves_the_created_pr() {
    issue_pr_flow(IssuePrChoice::Squash);
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_regular_and_draft_prs() {
    issue_pr_flow(IssuePrChoice::Ordinary);
    issue_pr_flow(IssuePrChoice::Draft);
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg(unix)]
enum IssuePrChoice {
    Ordinary,
    Draft,
    Squash,
}

#[cfg(unix)]
fn issue_pr_flow(choice: IssuePrChoice) {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([
        HttpResponse::tool_call(
            "issue-write",
            "write_file",
            serde_json::json!({"path":"implemented.txt","content":"implemented both issues"}),
        ),
        HttpResponse::streaming(["Issue implementation finished"], None),
    ]);
    fixture.write_config(&server.base_url());
    fixture.install_issue_provider();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["config", "user.name", "Issue Test"],
        vec!["config", "user.email", "issue@example.invalid"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["config", "core.hooksPath", "/dev/null"],
        vec![
            "remote",
            "add",
            "origin",
            "https://github.com/team/repo.git",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(fixture.workspace())
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(fixture.workspace().join("base.txt"), "base").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "."])
            .current_dir(fixture.workspace())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-c", "commit.gpgsign=false", "commit", "-m", "base"])
            .current_dir(fixture.workspace())
            .status()
            .unwrap()
            .success()
    );
    fixture.prepare_issue_remote();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("\u{2190} for agents");
    process.submit("/issue");
    process.wait_for_screen("Repair first issue");
    process.space();
    process.down();
    process.space();
    process.type_text("m");
    process.wait_for_screen("[issue #3] [issue #5]");
    process.type_text("implement together");
    process.enter();
    process.wait_for_screen("Approval required");
    process.enter();
    process.wait_for_transcript("Issue implementation finished");
    process.submit("/pr");
    process.wait_for_screen("automatically squash");
    for _ in 0..match choice {
        IssuePrChoice::Ordinary => 0,
        IssuePrChoice::Draft => 1,
        IssuePrChoice::Squash => 3,
    } {
        process.down();
    }
    process.enter();
    process.wait_for_screen("https://github.com/team/repo/pull/7");
    if choice == IssuePrChoice::Squash {
        process.wait_for_screen("Fixture rejected auto-merge");
        process.enter();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if fixture
                .find_file("auto-merge-count")
                .is_some_and(|path| fs::read_to_string(path).is_ok_and(|count| count == "2"))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the retry must reach automatic merge again"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        process.wait_for_screen("Fixture rejected auto-merge");
    }
    process.escape();
    process.quit();
    let count = fixture.find_file("create-count").unwrap();
    assert_eq!(fs::read_to_string(count).unwrap(), "1");
    let body: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.find_file("submitted-pr.json").unwrap()).unwrap())
            .unwrap();
    assert_eq!(body["base"], "main");
    assert_eq!(body["draft"], choice == IssuePrChoice::Draft);
    assert!(body["body"].as_str().unwrap().contains("issues/3"));
    assert!(body["body"].as_str().unwrap().contains("issues/5"));
    if choice == IssuePrChoice::Squash {
        let automatic: serde_json::Value = serde_json::from_slice(
            &fs::read(fixture.find_file("auto-merge.json").unwrap()).unwrap(),
        )
        .unwrap();
        let arguments = automatic["args"].as_array().unwrap();
        assert!(arguments.contains(&serde_json::json!("--auto")));
        assert!(arguments.contains(&serde_json::json!("--squash")));
        assert!(arguments.contains(&serde_json::json!("--match-head-commit")));
    } else {
        assert!(fixture.find_file("auto-merge.json").is_none());
    }
    assert!(!fixture.workspace().join("implemented.txt").exists());
    assert!(
        server
            .request_bodies()
            .first()
            .unwrap()
            .contains("First requirement")
    );
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_browser_paginates_searches_caches_refreshes_and_recovers_after_restart() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    fixture.install_issue_provider();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["config", "user.name", "Issue Test"],
        vec!["config", "user.email", "issue@example.invalid"],
        vec![
            "remote",
            "add",
            "origin",
            "https://github.com/team/repo.git",
        ],
        vec![
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "base",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(fixture.workspace())
                .status()
                .unwrap()
                .success()
        );
    }
    let list_path = fixture.find_file("issues.json").unwrap();
    let bin = list_path.parent().unwrap().to_owned();
    let rows = (1..=2300).map(|number| serde_json::json!({"number":number,"title":format!("Paged issue {number}"),"body":null,"html_url":format!("https://github.com/team/repo/issues/{number}"),"updated_at":"now","state":"open"})).collect::<Vec<_>>();
    fs::write(&list_path, serde_json::to_vec(&rows).unwrap()).unwrap();
    let count = || {
        fs::read_to_string(bin.join("issue-requests.jsonl"))
            .unwrap()
            .lines()
            .filter(|line| {
                line.contains("/issues?")
                    || line.contains("search/issues?")
                    || line.contains("/issues/5001")
            })
            .count()
    };
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/issue");
    process.wait_for_screen("100 loaded");
    assert_eq!(count(), 1);
    process.type_text("n");
    process.wait_for_screen("200 loaded");
    assert_eq!(count(), 2);
    process.send(b"\x1b[F");
    process.right();
    process.wait_for_screen("Previously resolved issue");
    assert_eq!(count(), 3);
    process.left();
    process.send(b"\x1b[H");
    process.down();
    assert!(
        process.screen().contains("200 loaded"),
        "an Issue changing groups must not be duplicated"
    );
    assert_eq!(
        count(),
        3,
        "open pages remain loaded when the closed group folds"
    );
    process.type_text("/memory leak");
    process.enter();
    process.wait_for_screen("Search found issue outside the first page");
    assert_eq!(count(), 4);
    process.type_text("/");
    process.send(b"\x15");
    process.type_text("#5001");
    process.enter();
    process.wait_for_screen("Repair issue outside loaded pages");
    assert_eq!(count(), 5);
    process.type_text("r");
    process.wait_for_stable_screen("Updated");
    assert_eq!(count(), 6);
    fs::write(bin.join("issue-offline"), "offline").unwrap();
    process.type_text("r");
    process.wait_for_screen("Fixture offline");
    assert!(
        process
            .screen()
            .contains("Repair issue outside loaded pages")
    );
    fs::remove_file(bin.join("issue-offline")).unwrap();
    process.send(b"\x12");
    process.wait_for_stable_screen("Updated");
    assert_eq!(count(), 8);
    process.escape();
    process.submit("/issue");
    process.wait_for_screen("100 loaded");
    assert_eq!(
        count(),
        9,
        "clearing search cache also clears other pages of this repository"
    );
    process.escape();
    process.quit();
    let mut restarted = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    restarted.wait_for_screen("Zeta Code v");
    restarted.submit("/issue");
    restarted.wait_for_screen("Cached");
    assert_eq!(count(), 9, "restart must reuse persisted cache");
    restarted.escape();
    restarted.quit();
    let database = fixture.find_file("state.sqlite3").unwrap();
    assert!(Command::new("python3").args(["-c", "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('UPDATE issue_pages SET fetched_at=fetched_at-601'); c.commit()"]).arg(&database).status().unwrap().success());
    let mut expired = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    expired.wait_for_screen("Zeta Code v");
    expired.submit("/issue");
    expired.wait_for_screen("Cached");
    expired.wait_for_screen("Updated");
    assert_eq!(
        count(),
        10,
        "expired cache is displayed before the visible list refreshes"
    );
    expired.escape();
    expired.quit();
    let config_path = fixture.find_file("config.toml").unwrap();
    let source = fs::read_to_string(&config_path).unwrap();
    let source = if source.contains("autoRefreshMinutes = 10") {
        source.replace("autoRefreshMinutes = 10", "autoRefreshMinutes = 0")
    } else {
        format!("{source}\n[issues]\nautoRefreshMinutes = 0\n")
    };
    fs::write(&config_path, source).unwrap();
    assert!(Command::new("python3").args(["-c", "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('UPDATE issue_pages SET fetched_at=fetched_at-601'); c.commit()"]).arg(database).status().unwrap().success());
    let mut never = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    never.wait_for_screen("Zeta Code v");
    never.submit("/issue");
    never.wait_for_stable_screen("Cached");
    assert_eq!(
        count(),
        10,
        "Never must reuse stale cache without refreshing"
    );
    never.escape();
    never.quit();
    assert!(server.request_bodies().is_empty());
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_workflow_creates_labels_claims_and_links_a_branch() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([]);
    fixture.write_config(&server.base_url());
    fixture.install_issue_provider();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["config", "user.name", "Issue Test"],
        vec!["config", "user.email", "issue@example.invalid"],
        vec![
            "remote",
            "add",
            "origin",
            "https://github.com/team/repo.git",
        ],
        vec![
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "base",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(fixture.workspace())
                .status()
                .unwrap()
                .success()
        );
    }
    fixture.prepare_issue_remote();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/issue");
    process.wait_for_screen("Repair first issue");
    process.type_text("w");
    process.wait_for_screen("GitHub assignee");
    for _ in 0..5 {
        process.down();
    }
    process.enter();
    process.type_text("tester");
    process.enter();
    process.type_text("l");
    process.wait_for_screen("5 labels");
    process.type_text("s");
    wait_for_config(&fixture, "assignee = \"tester\"");
    process.escape();
    process.wait_for_screen("─ Issues");
    process.wait_for_screen("Repair first issue");
    process.space();
    process.type_text("g");
    process.wait_for_screen("Review issue assignments");
    process.wait_for_screen("combined");
    process.type_text("c");
    process.wait_for_screen("Queued");
    process.wait_for_screen("@tester");
    let labels: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.find_file("3.json").unwrap()).unwrap()).unwrap();
    assert_eq!(labels["assignees"][0]["login"], "tester");
    assert_eq!(labels["labels"][0]["name"], "status:queued");
    let linked: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.find_file("linked-branches.json").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(linked["ISSUE_3"].as_array().unwrap().len(), 1);
    assert!(
        linked["ISSUE_3"][0]["ref"]["name"]
            .as_str()
            .unwrap()
            .starts_with("codex/issue-3-")
    );
    let issue_path = fixture.find_file("3.json").unwrap();
    let mut closed: serde_json::Value =
        serde_json::from_slice(&fs::read(&issue_path).unwrap()).unwrap();
    closed["state"] = "closed".into();
    fs::write(&issue_path, serde_json::to_vec(&closed).unwrap()).unwrap();
    process.type_text("y");
    process.wait_for_screen("closed or changed identity");
    closed["state"] = "open".into();
    fs::write(&issue_path, serde_json::to_vec(&closed).unwrap()).unwrap();
    process.type_text("y");
    process.wait_for_stable_screen("Idle");
    process.type_text("t");
    process.send(b"\x15");
    process.type_text("other");
    process.enter();
    process.wait_for_screen("@other");
    process.wait_for_screen("Paused");
    process.type_text("u");
    process.wait_for_screen("Released");
    let released: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.find_file("3.json").unwrap()).unwrap()).unwrap();
    assert!(released["assignees"].as_array().unwrap().is_empty());
    assert_eq!(released["labels"][0]["name"], "status:todo");
    process.escape();
    process.wait_for_screen("─ Issues");
    process.wait_for_screen("2 loaded");
    process.type_text("w");
    process.wait_for_screen("GitHub assignee");
    for _ in 0..17 {
        process.down();
    }
    process.enter();
    process.send(b"\x15");
    process.type_text("true");
    process.enter();
    process.type_text("s");
    wait_for_config(&fixture, "maxIssues = 1");
    process.escape();
    process.wait_for_screen("─ Issues");
    process.wait_for_screen("2 loaded");

    process.wait_for_screen("Queued");
    process.wait_for_screen("@tester");
    process.escape();
    process.quit();
    assert!(
        server.request_bodies().is_empty(),
        "claiming does not invoke an implementation model"
    );
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_assignment_executes_verifies_and_creates_its_pr() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([
        HttpResponse::tool_call(
            "issue-write",
            "write_file",
            serde_json::json!({"path":"implemented.txt","content":"verified issue implementation"}),
        ),
        HttpResponse::tool_call(
            "issue-goal",
            "update_goal",
            serde_json::json!({"status":"complete"}),
        ),
        HttpResponse::streaming(["Issue worker finished"], None),
    ]);
    let bin = prepare_issue_execution_fixture(&fixture, &server);
    let config_path = fixture.find_file("config.toml").unwrap();
    let config = fs::read_to_string(&config_path).unwrap().replace(
        "delivery = \"pullRequest\"",
        "delivery = \"pullRequest\"\nautoMerge = true",
    );
    fs::write(config_path, config).unwrap();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    process.space();
    process.type_text("g");
    process.wait_for_screen("combined");
    process.type_text("s");
    process.wait_for_screen("─ Issues");
    open_issue_details(&mut process, 3);
    process.enter();
    process.wait_for_screen("Approval required");
    process.enter();
    process.wait_for_transcript("Issue worker finished");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    open_issue_details(&mut process, 3);
    process.wait_for_stable_screen("Review");
    process.type_text("v");
    process.wait_for_screen("Verified");
    let target = Command::new("git")
        .args(["rev-parse", "refs/remotes/origin/main"])
        .current_dir(fixture.workspace())
        .output()
        .unwrap();
    let target = String::from_utf8(target.stdout).unwrap();
    let target = target.trim();
    let advanced = Command::new("git")
        .args([
            "commit-tree",
            &format!("{target}^{{tree}}"),
            "-p",
            target,
            "-m",
            "Upstream update after verification",
        ])
        .current_dir(fixture.workspace())
        .output()
        .unwrap();
    assert!(advanced.status.success());
    let advanced = String::from_utf8(advanced.stdout).unwrap();
    assert!(
        Command::new("git")
            .arg("push")
            .arg(bin.join("origin.git"))
            .arg(format!("{}:refs/heads/main", advanced.trim()))
            .current_dir(fixture.workspace())
            .status()
            .unwrap()
            .success()
    );
    process.type_text("d");
    process.wait_for_screen("Target branch changed");
    assert!(!bin.join("pull.json").exists());
    process.type_text("v");
    process.wait_for_screen("Verified");
    process.type_text("d");
    process.wait_for_screen("PR awaiting merge");
    assert_eq!(
        fs::read_to_string(bin.join("auto-merge-count")).unwrap(),
        "1"
    );
    let database = fixture.find_file("state.sqlite3").unwrap();
    assert!(Command::new("python3").args(["-c", "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute(\"UPDATE issue_assignments SET value=json_set(value, '$.delivery.pullRequestNumber', NULL, '$.delivery.pullRequestUrl', NULL)\"); c.commit()"]).arg(&database).status().unwrap().success());
    let pull_path = fixture.find_file("pull.json").unwrap();
    let mut merged: serde_json::Value =
        serde_json::from_slice(&fs::read(&pull_path).unwrap()).unwrap();
    merged["state"] = "closed".into();
    merged["merged_at"] = "2026-09-08T00:00:00Z".into();
    assert!(
        Command::new("git")
            .arg("--git-dir")
            .arg(bin.join("origin.git"))
            .args([
                "update-ref",
                "refs/heads/main",
                merged["head"]["sha"].as_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    fs::write(pull_path, serde_json::to_vec(&merged).unwrap()).unwrap();
    process.wait_for_screen("Delivered");
    let completed: serde_json::Value =
        serde_json::from_slice(&fs::read(bin.join("3.json")).unwrap()).unwrap();
    assert!(Command::new("python3").args(["-c", "import sqlite3,sys,json; c=sqlite3.connect(sys.argv[1]); v=json.loads(c.execute('SELECT value FROM issue_assignments').fetchone()[0]); assert v['delivery']['pullRequestNumber']==7 and v['ownership']=='completed'"]).arg(&database).status().unwrap().success());
    assert_eq!(completed["state"], "closed");
    assert!(completed["labels"].as_array().unwrap().is_empty());
    process.escape();
    process.quit();
    assert_eq!(
        fs::read_to_string(fixture.find_file("create-count").unwrap()).unwrap(),
        "1"
    );
    let pr: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.find_file("submitted-pr.json").unwrap()).unwrap())
            .unwrap();
    assert!(pr["body"].as_str().unwrap().contains("Closes #3"));
    assert_eq!(pr["base"], "main");
    assert!(!fixture.workspace().join("implemented.txt").exists());
}

#[cfg(unix)]
fn prepare_issue_execution_fixture(
    fixture: &Fixture,
    server: &ScenarioServer,
) -> std::path::PathBuf {
    fixture.write_config(&server.base_url());
    fixture.install_issue_provider();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["config", "user.name", "Issue Test"],
        vec!["config", "user.email", "issue@example.invalid"],
        vec![
            "remote",
            "add",
            "origin",
            "https://github.com/team/repo.git",
        ],
        vec![
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "base",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(fixture.workspace())
                .status()
                .unwrap()
                .success()
        );
    }
    fixture.prepare_issue_remote();
    let config_path = fixture.find_file("config.toml").unwrap();
    let mut config = fs::read_to_string(&config_path).unwrap();
    config.push_str(
        r#"
[issues.repositories."github.com:REPO_fixture"]
assignee = "tester"
branchTemplate = "codex/issue-{number}-{slug}-{attempt}"
publication = "linked"
coordinatorAgent = ""
workerAgent = ""
maxParallel = 3
maxTokens = 100000
delivery = "pullRequest"
closeOnCompletion = true
validationCommands = ["test \"$(cat implemented.txt)\" = \"verified issue implementation\""]
[issues.repositories."github.com:REPO_fixture".labels]
todo = "status:todo"
queued = "status:queued"
inProgress = "status:in-progress"
review = "status:review"
blocked = "status:blocked"
"#,
    );
    fs::write(config_path, config).unwrap();
    let bin = fixture
        .find_file("3.json")
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let labels = ["todo", "queued", "in-progress", "review", "blocked"].into_iter().map(|stage| serde_json::json!({"name":format!("status:{stage}"),"color":"123456","node_id":format!("LABEL_{stage}")})).collect::<Vec<_>>();
    fs::write(
        bin.join("labels.json"),
        serde_json::to_vec(&labels).unwrap(),
    )
    .unwrap();
    bin
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_pause_cancels_validation_without_publishing() {
    let fixture = Fixture::new();
    let server = ScenarioServer::start([
        HttpResponse::tool_call(
            "write",
            "write_file",
            serde_json::json!({"path":"implemented.txt","content":"verified issue implementation"}),
        ),
        HttpResponse::tool_call(
            "goal",
            "update_goal",
            serde_json::json!({"status":"complete"}),
        ),
        HttpResponse::streaming(["Worker ready"], None),
    ]);
    let bin = prepare_issue_execution_fixture(&fixture, &server);
    let config_path = fixture.find_file("config.toml").unwrap();
    let config = fs::read_to_string(&config_path).unwrap().replace(
        "validationCommands = [",
        "validationCommands = [\"sleep 60\", ",
    );
    fs::write(config_path, config).unwrap();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    process.space();
    process.type_text("g");
    process.wait_for_screen("combined");
    process.type_text("s");
    process.wait_for_screen("─ Issues");
    open_issue_details(&mut process, 3);
    process.enter();
    process.wait_for_screen("Approval required");
    process.enter();
    process.wait_for_transcript("Worker ready");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    open_issue_details(&mut process, 3);
    process.wait_for_stable_screen("Review");
    process.type_text("v");
    process.wait_for_screen("Loading...");
    let started = std::time::Instant::now();
    process.type_text("p");
    process.wait_for_screen("Paused");
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    assert!(!bin.join("pull.json").exists());
    assert!(!fixture.workspace().join("implemented.txt").exists());
    process.escape();
    process.quit();
}

#[test]
#[cfg(unix)]
fn actual_tui_issue_distribution_runs_isolated_workers_and_verifies_the_combination() {
    let fixture = Fixture::new();
    let plan = serde_json::json!([
        {"id":"first", "numbers":[3], "objective":"Produce the first result", "acceptanceConditions":["first.txt contains verified issue implementation"], "scope":{"components":[],"paths":["first.txt"],"contracts":[],"resources":[]}, "dependencies":[]},
        {"id":"second", "numbers":[5], "objective":"Produce the second result", "acceptanceConditions":["second.txt contains verified issue implementation"], "scope":{"components":[],"paths":["second.txt"],"contracts":[],"resources":[]}, "dependencies":[]}
    ]).to_string();
    let server = ScenarioServer::start([
        HttpResponse::Json { body: serde_json::json!({"id":"planning-response","object":"chat.completion","created":0,"model":"zeta-real-scenario","choices":[{"index":0,"message":{"role":"assistant","content":plan},"finish_reason":"stop"}],"usage":{"prompt_tokens":11,"completion_tokens":7}}).to_string().into_bytes() },
        HttpResponse::tool_call("write-second", "write_file", serde_json::json!({"path":"second.txt","content":"verified issue implementation"})),
        HttpResponse::tool_call("goal-second", "update_goal", serde_json::json!({"status":"complete"})),
        HttpResponse::streaming(["Second worker ready"], None),
        HttpResponse::tool_call("write-first", "write_file", serde_json::json!({"path":"first.txt","content":"verified issue implementation"})),
        HttpResponse::tool_call("goal-first", "update_goal", serde_json::json!({"status":"complete"})),
        HttpResponse::streaming(["First worker ready"], None),
    ]);
    prepare_issue_execution_fixture(&fixture, &server);
    let path = fixture.find_file("config.toml").unwrap();
    let config = fs::read_to_string(&path)
        .unwrap()
        .replace("maxParallel = 3", "maxParallel = 1");
    let validation = r#"validationCommands = ["test -f first.txt || test -f second.txt", "for f in first.txt second.txt; do if test -f $f; then test \"$(cat $f)\" = \"verified issue implementation\" || exit 1; fi; done"]"#;
    fs::write(
        path,
        config
            .lines()
            .map(|line| {
                if line.starts_with("validationCommands =") {
                    validation
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    let mut process = TuiProcess::start(&fixture, &[], LARGE_SIZE);
    process.wait_for_screen("Zeta Code v");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    process.space();
    process.down();
    process.space();
    process.type_text("d");
    process.wait_for_screen("Produce the second result");
    process.type_text("s");
    process.wait_for_screen("─ Issues");
    open_issue_details(&mut process, 5);
    process.enter();
    process.wait_for_screen("Approval required");
    process.enter();
    process.wait_for_transcript("Second worker ready");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    open_issue_details(&mut process, 3);
    process.enter();
    process.wait_for_screen("Approval required");
    process.enter();
    process.wait_for_transcript("First worker ready");
    process.submit("/issue");
    process.wait_for_screen("2 loaded");
    process.wait_for_screen("Review (2)");
    open_issue_details(&mut process, 5);
    process.type_text("v");
    process.wait_for_screen("Verified");
    let branches = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/codex/issue-batch-*",
        ])
        .current_dir(fixture.workspace())
        .output()
        .unwrap();
    let branch = String::from_utf8(branches.stdout).unwrap();
    let branch = branch.trim();
    assert!(!branch.is_empty());
    for file in ["first.txt", "second.txt"] {
        let result = Command::new("git")
            .args(["show", &format!("{branch}:{file}")])
            .current_dir(fixture.workspace())
            .output()
            .unwrap();
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            "verified issue implementation"
        );
        assert!(!fixture.workspace().join(file).exists());
    }
    process.escape();
    process.quit();
}

#[cfg(unix)]
fn open_issue_details(process: &mut TuiProcess, number: u64) {
    process.type_text("/");
    process.send(b"\x15");
    process.type_text(&format!("#{number}"));
    process.enter();
    process.wait_for_screen("1 loaded");
    process.enter();
    process.wait_for_screen("Issue details");
}
