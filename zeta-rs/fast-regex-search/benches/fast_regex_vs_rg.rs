use std::collections::BTreeSet;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use zeta_fast_regex_search::FastRegexCaseSensitivity;
use zeta_fast_regex_search::FastRegexPattern;
use zeta_fast_regex_search::FastRegexQuery;
use zeta_fast_regex_search::FastRegexSearch;
use zeta_fast_regex_search::FastRegexSearchLimits;
use zeta_fast_regex_search::FastRegexSearchStorage;
use zeta_workspace::WorkspaceRoot;

const DEFAULT_FILE_COUNT: usize = 8_000;
const DEFAULT_RUN_COUNT: usize = 15;

#[derive(Clone, Copy)]
struct BenchmarkCase {
    name: &'static str,
    pattern: &'static str,
}

struct Measurement {
    source: &'static str,
    name: &'static str,
    candidates: usize,
    matches: usize,
    fast_median: Duration,
    rg_median: Duration,
}

struct RipgrepResult {
    paths: BTreeSet<PathBuf>,
    matching_lines: usize,
}

fn main() {
    let require_faster = std::env::args().any(|argument| argument == "--require-faster");
    let file_count = environment_usize("FAST_REGEX_BENCH_FILES", DEFAULT_FILE_COUNT);
    let run_count = environment_usize("FAST_REGEX_BENCH_RUNS", DEFAULT_RUN_COUNT);
    let rg = std::env::var_os("RG").unwrap_or_else(|| "rg".into());
    let workspace = build_corpus(file_count);
    let storage = tempfile::tempdir().expect("benchmark index storage");
    let max_results = file_count.saturating_mul(25).saturating_add(100);
    let limits = FastRegexSearchLimits {
        max_files: file_count + 10,
        max_results,
        max_total_source_bytes: 2 * 1024 * 1024 * 1024,
        ..FastRegexSearchLimits::default()
    };
    let index = FastRegexSearch::open(
        WorkspaceRoot::open(workspace.path()).expect("benchmark workspace root"),
        FastRegexSearchStorage::Persistent(storage.path().to_path_buf()),
        limits.clone(),
    )
    .expect("open fast regex index");
    let build_started = Instant::now();
    let snapshot = index.rebuild().expect("build fast regex index");
    let build_elapsed = build_started.elapsed();
    let cases = [
        BenchmarkCase {
            name: "rare-prefix-suffix",
            pattern: r"workspace_authentication_[0-9]+_token",
        },
        BenchmarkCase {
            name: "rare-alternation",
            pattern: r"(?:alpha|beta)_rare_handler",
        },
        BenchmarkCase {
            name: "rare-no-match",
            pattern: r"missing_workspace_dispatch_.*_completion",
        },
        BenchmarkCase {
            name: "unselective-short",
            pattern: r"fn",
        },
    ];
    let mut measurements = Vec::new();
    for case in cases {
        measurements.push(measure_case(
            "built",
            &index,
            workspace.path(),
            &rg,
            max_results,
            run_count,
            case,
        ));
    }

    let changed_path = workspace.path().join("src/batch-0/module-1.rs");
    let mut changed_content = fs::read_to_string(&changed_path).expect("changed benchmark source");
    changed_content.push_str("fn incremental_refresh_marker() {}\n");
    fs::write(&changed_path, changed_content).expect("update benchmark source");
    let refresh_started = Instant::now();
    index
        .refresh_observed_paths(std::slice::from_ref(&changed_path))
        .expect("benchmark incremental refresh");
    let refresh_elapsed = refresh_started.elapsed();
    drop(index);
    let reopen_started = Instant::now();
    let reopened = FastRegexSearch::open(
        WorkspaceRoot::open(workspace.path()).expect("reopened benchmark workspace root"),
        FastRegexSearchStorage::Persistent(storage.path().to_path_buf()),
        limits,
    )
    .expect("reopen fast regex index");
    let reopen_elapsed = reopen_started.elapsed();
    assert_ne!(reopened.snapshot().generation, 0);
    for case in cases {
        measurements.push(measure_case(
            "reopened",
            &reopened,
            workspace.path(),
            &rg,
            max_results,
            run_count,
            case,
        ));
    }

    println!(
        "indexed {} files / {} MiB in {:.3}s",
        snapshot.indexed_file_count,
        snapshot.indexed_source_bytes / (1024 * 1024),
        build_elapsed.as_secs_f64()
    );
    println!(
        "one-file refresh: {:.3} ms; validated reopen: {:.3} ms",
        milliseconds(refresh_elapsed),
        milliseconds(reopen_elapsed)
    );
    println!(
        "{:<9} {:<24} {:>11} {:>9} {:>12} {:>12} {:>9}",
        "state", "case", "candidates", "matches", "fast median", "rg median", "ratio"
    );
    for measurement in &measurements {
        println!(
            "{:<9} {:<24} {:>11} {:>9} {:>9.3} ms {:>9.3} ms {:>8.2}x",
            measurement.source,
            measurement.name,
            measurement.candidates,
            measurement.matches,
            milliseconds(measurement.fast_median),
            milliseconds(measurement.rg_median),
            measurement.rg_median.as_secs_f64() / measurement.fast_median.as_secs_f64(),
        );
    }
    if require_faster {
        let failures = measurements
            .iter()
            .filter(|measurement| measurement.fast_median >= measurement.rg_median)
            .map(|measurement| measurement.name)
            .collect::<Vec<_>>();
        assert!(
            failures.is_empty(),
            "fast regex did not beat rg: {}",
            failures.join(", ")
        );
    }
}

fn measure_case(
    source: &'static str,
    index: &FastRegexSearch,
    root: &Path,
    rg: &std::ffi::OsStr,
    max_results: usize,
    run_count: usize,
    case: BenchmarkCase,
) -> Measurement {
    let query = FastRegexQuery {
        query: case.pattern.into(),
        pattern: FastRegexPattern::Regex,
        case_sensitivity: FastRegexCaseSensitivity::Sensitive,
        scope: PathBuf::new(),
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        max_results,
    };
    let fast_warmup = index.search(&query).expect("fast regex warmup");
    let rg_warmup = run_rg(root, rg, case.pattern);
    assert_eq!(matched_paths(&fast_warmup.matches), rg_warmup.paths);
    assert_eq!(
        fast_warmup.matches.len(),
        rg_warmup.matching_lines,
        "matching-line count differs from rg for {}",
        case.name
    );
    let mut fast_samples = Vec::with_capacity(run_count);
    let mut rg_samples = Vec::with_capacity(run_count);
    for _ in 0..run_count {
        let started = Instant::now();
        let result = index.search(black_box(&query)).expect("fast regex query");
        black_box(result);
        fast_samples.push(started.elapsed());

        let started = Instant::now();
        black_box(run_rg(root, rg, black_box(case.pattern)));
        rg_samples.push(started.elapsed());
    }
    Measurement {
        source,
        name: case.name,
        candidates: fast_warmup.statistics.candidate_file_count,
        matches: fast_warmup.matches.len(),
        fast_median: median(&mut fast_samples),
        rg_median: median(&mut rg_samples),
    }
}

fn run_rg(root: &Path, rg: &std::ffi::OsStr, pattern: &str) -> RipgrepResult {
    let output = Command::new(rg)
        .current_dir(root)
        .args(["--no-config", "--no-heading", "--line-number", "--"])
        .arg(pattern)
        .arg(".")
        .output()
        .expect("execute rg benchmark baseline");
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "rg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout)
        .expect("rg output is UTF-8")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let paths = output
        .iter()
        .map(|line| line.split_once(':').expect("rg path separator").0)
        .map(|path| path.strip_prefix("./").unwrap_or(path))
        .map(PathBuf::from)
        .collect();
    RipgrepResult {
        paths,
        matching_lines: output.len(),
    }
}

fn matched_paths(matches: &[zeta_fast_regex_search::FastRegexMatch]) -> BTreeSet<PathBuf> {
    matches.iter().map(|found| found.path.clone()).collect()
}

fn build_corpus(file_count: usize) -> TempDir {
    let directory = tempfile::tempdir().expect("benchmark directory");
    fs::create_dir(directory.path().join(".git")).expect("git marker");
    for file_index in 0..file_count {
        let relative = PathBuf::from(format!(
            "src/batch-{}/module-{file_index}.rs",
            file_index / 100
        ));
        let absolute = directory.path().join(relative);
        fs::create_dir_all(absolute.parent().expect("module parent")).expect("module directory");
        let mut content = String::with_capacity(2_048);
        for line_index in 0..24 {
            content.push_str(&format!(
                "fn common_workspace_function_{line_index}(value: usize) -> usize {{ value + {file_index} }}\n"
            ));
        }
        if file_index % 1_997 == 0 {
            content.push_str(&format!(
                "let workspace_authentication_{file_index}_token = authorize();\n"
            ));
        }
        if file_index % 2_003 == 0 {
            let branch = if file_index % 2 == 0 { "alpha" } else { "beta" };
            content.push_str(&format!("fn {branch}_rare_handler() {{}}\n"));
        }
        fs::write(absolute, content).expect("benchmark source");
    }
    directory
}

fn environment_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn median(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
