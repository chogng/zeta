use std::fs;
use std::path::Path;
use std::path::PathBuf;

const MAX_PRODUCTION_MODULE_LINES: usize = 500;

struct LayerRule {
    directory: &'static str,
    allowed_crate_paths: &'static [&'static str],
}

const LAYER_RULES: &[LayerRule] = &[
    LayerRule {
        directory: "foundation",
        allowed_crate_paths: &[],
    },
    LayerRule {
        directory: "layout",
        allowed_crate_paths: &["crate::foundation::"],
    },
    LayerRule {
        directory: "text",
        allowed_crate_paths: &["crate::foundation::", "crate::text::"],
    },
    LayerRule {
        directory: "presentation",
        allowed_crate_paths: &["crate::foundation::", "crate::text::"],
    },
    LayerRule {
        directory: "runtime",
        allowed_crate_paths: &["crate::foundation::"],
    },
];

#[test]
fn framework_sources_are_physically_partitioned_by_layer() {
    let source_root = source_root();
    let mut unexpected = Vec::new();
    for entry in fs::read_dir(&source_root).expect("zui source root should be readable") {
        let path = entry.expect("zui source entry should be readable").path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !LAYER_RULES.iter().any(|rule| rule.directory == name) {
                unexpected.push(path.display().to_string());
            }
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if !["architecture_tests.rs", "lib.rs", "renderer_support.rs"].contains(&name) {
            unexpected.push(path.display().to_string());
        }
    }
    assert!(
        unexpected.is_empty(),
        "zui source must live in a declared framework layer; unexpected paths:\n{}",
        unexpected.join("\n")
    );
}

#[test]
fn framework_layers_only_depend_in_the_declared_direction() {
    let source_root = source_root();
    let mut violations = Vec::new();
    for rule in LAYER_RULES {
        for path in production_rust_files(&source_root.join(rule.directory)) {
            let source = fs::read_to_string(&path).expect("zui source should be readable");
            for (index, line) in source.lines().enumerate() {
                let Some(crate_path) = line.find("crate::").map(|offset| &line[offset..]) else {
                    continue;
                };
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if rule
                    .allowed_crate_paths
                    .iter()
                    .any(|allowed| crate_path.starts_with(allowed))
                {
                    continue;
                }
                violations.push(format!(
                    "{}:{} crosses the {} layer boundary with `{}`",
                    path.display(),
                    index + 1,
                    rule.directory,
                    line.trim()
                ));
            }
        }
    }

    let renderer_support = source_root.join("renderer_support.rs");
    let source = fs::read_to_string(&renderer_support).expect("renderer bridge should be readable");
    for (index, line) in source.lines().enumerate() {
        let Some(crate_path) = line.find("crate::").map(|offset| &line[offset..]) else {
            continue;
        };
        if line.trim_start().starts_with("//") || crate_path.starts_with("crate::text::") {
            continue;
        }
        violations.push(format!(
            "{}:{} renderer bridge may only consume text contracts: `{}`",
            renderer_support.display(),
            index + 1,
            line.trim()
        ));
    }

    assert!(
        violations.is_empty(),
        "zui internal dependencies must flow foundation → layout/text → presentation, while runtime stays presentation-independent:\n{}",
        violations.join("\n")
    );
}

#[test]
fn framework_layers_do_not_absorb_platform_or_product_owners() {
    let source_root = source_root();
    let forbidden = [
        "accesskit::",
        "wgpu::",
        "winit::",
        "zeta_native",
        "zeta_ui::",
    ];
    let mut violations = Vec::new();
    for path in production_rust_files(&source_root) {
        let source = fs::read_to_string(&path).expect("zui source should be readable");
        for token in forbidden {
            if source.contains(token) {
                violations.push(format!("{} contains `{token}`", path.display()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "platform adapters, graphics backends, components, and product state must remain above zui:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_modules_stay_small_and_the_root_facade_stays_explicit() {
    let source_root = source_root();
    let mut oversized = Vec::new();
    for path in production_rust_files(&source_root) {
        let source = fs::read_to_string(&path).expect("zui source should be readable");
        let line_count = source.lines().count();
        if line_count > MAX_PRODUCTION_MODULE_LINES {
            oversized.push(format!("{} has {line_count} lines", path.display()));
        }
    }
    assert!(
        oversized.is_empty(),
        "zui production modules must stay under {MAX_PRODUCTION_MODULE_LINES} lines; extract an owned submodule instead of extending:\n{}",
        oversized.join("\n")
    );

    let facade =
        fs::read_to_string(source_root.join("lib.rs")).expect("zui facade should be readable");
    assert!(
        !facade.lines().any(|line| line.contains("::*;")),
        "zui's public facade must explicitly export every supported contract"
    );
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn production_rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_rust_files(root, &mut files);
    files.retain(|path| {
        !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("_tests.rs") || name == "architecture_tests.rs")
    });
    files.sort();
    files
}

fn visit_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "could not read zui source directory {}: {error}",
            root.display()
        )
    }) {
        let path = entry.expect("zui source entry should be readable").path();
        if path.is_dir() {
            visit_rust_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}
