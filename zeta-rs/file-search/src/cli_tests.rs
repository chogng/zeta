use super::*;
use std::fs;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[test]
fn parses_codex_style_search_arguments() {
    let cli = Cli::try_parse_from([
        "zeta-file-search",
        "--json",
        "--limit",
        "7",
        "--threads",
        "1",
        "-C",
        "workspace",
        "--compute-indices",
        "src",
    ])
    .unwrap();

    assert!(cli.json);
    assert_eq!(cli.limit.get(), 7);
    assert_eq!(cli.threads.get(), 1);
    assert_eq!(cli.cwd, Some(PathBuf::from("workspace")));
    assert!(cli.compute_indices);
    assert_eq!(cli.pattern.as_deref(), Some("src"));
}

#[test]
fn json_output_uses_the_library_search_results() {
    let workspace = temporary_workspace();
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::write(workspace.join("src/lib.rs"), "contents").unwrap();
    fs::write(workspace.join("notes.md"), "contents").unwrap();
    let cli = Cli::try_parse_from([
        "zeta-file-search",
        "--json",
        "--compute-indices",
        "-C",
        workspace.to_str().unwrap(),
        "lib",
    ])
    .unwrap();
    let mut output = Vec::new();
    let mut warnings = Vec::new();

    execute(cli, &mut output, &mut warnings, TerminalOutput::Plain).unwrap();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["path"], "src/lib.rs");
    assert!(value["score"].as_u64().is_some());
    assert!(
        value["indices"]
            .as_array()
            .is_some_and(|indices| !indices.is_empty())
    );
    assert!(warnings.is_empty());
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn omitted_pattern_lists_workspace_files_in_plain_text() {
    let workspace = temporary_workspace();
    fs::write(workspace.join("alpha.rs"), "contents").unwrap();
    fs::write(workspace.join("beta.rs"), "contents").unwrap();
    let cli = Cli::try_parse_from(["zeta-file-search", "-C", workspace.to_str().unwrap()]).unwrap();
    let mut output = Vec::new();
    let mut warnings = Vec::new();

    execute(cli, &mut output, &mut warnings, TerminalOutput::Plain).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "alpha.rs\nbeta.rs\n");
    assert!(warnings.is_empty());
    let _ = fs::remove_dir_all(workspace);
}

fn temporary_workspace() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "zeta-file-search-cli-tests-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
