use crate::scenario_http::HttpResponse;
use crate::scenario_http::ScenarioServer;
use crate::tui_process::Fixture;
use crate::tui_process::LARGE_SIZE;
use crate::tui_process::SMALL_SIZE;
use crate::tui_process::TuiProcess;
use std::fs;
use std::process::Command;

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
    process.back_tab();
    process.right();
    process.wait_for_screen("Previously resolved issue");
    assert!(!process.screen().contains("Repair first issue"));
    process.resize(SMALL_SIZE);
    process.wait_for_screen("Previously resolved issue");
    process.resize(LARGE_SIZE);
    process.left();
    process.wait_for_screen("Repair first issue");
    assert!(!process.screen().contains("Previously resolved issue"));
    process.down();
    process.space();
    process.down();
    process.space();
    process.wait_for_screen("2 selected");
    process.resize(SMALL_SIZE);
    process.wait_for_screen("2 selected");
    process.resize(LARGE_SIZE);
    process.tab();
    process.enter();
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
enum IssuePrChoice { Ordinary, Draft, Squash }

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
    process.tab();
    process.tab();
    process.enter();
    process.wait_for_screen("[issue #3] [issue #5]");
    process.type_text("implement together");
    process.enter();
    process.wait_for_screen("Approval required");
    process.enter();
    process.wait_for_transcript("Issue implementation finished");
    process.submit("/pr");
    process.wait_for_screen("automatically squash");
    for _ in 0..match choice { IssuePrChoice::Ordinary => 0, IssuePrChoice::Draft => 1, IssuePrChoice::Squash => 3 } { process.down(); }
    process.enter();
    process.wait_for_screen("https://github.com/team/repo/pull/7");
    if choice == IssuePrChoice::Squash {
        process.wait_for_screen("Fixture rejected auto-merge");
        process.enter();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if fixture.find_file("auto-merge-count").is_some_and(|path| fs::read_to_string(path).is_ok_and(|count| count == "2")) { break; }
            assert!(std::time::Instant::now() < deadline, "the retry must reach automatic merge again");
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
        let automatic: serde_json::Value = serde_json::from_slice(&fs::read(fixture.find_file("auto-merge.json").unwrap()).unwrap()).unwrap();
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
