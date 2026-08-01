use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn product_composition_uses_scene_draw_component() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    visit_rust_sources(&source_root, &mut |path, source| {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("_tests.rs"))
        {
            return;
        }
        for forbidden in [".paint(scene)", ".paint(&mut scene)"] {
            if source.contains(forbidden) {
                violations.push(format!("{} contains `{forbidden}`", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "Native product composition must use UiScene::draw_component so inspection ancestry is preserved:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_contract_does_not_depend_on_a_gpu_backend() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("native crate should be inside the Rust workspace");
    let ui_root = workspace.join("ui");
    let manifest = fs::read_to_string(ui_root.join("Cargo.toml"))
        .expect("zeta-ui manifest should be readable");
    for forbidden in ["wgpu", "glyphon"] {
        assert!(
            !manifest
                .lines()
                .any(|line| line.trim_start().starts_with(forbidden)),
            "zeta-ui must not depend on {forbidden}; GPU dependencies belong to backend crates"
        );
    }

    let mut violations = Vec::new();
    visit_rust_sources(&ui_root.join("src"), &mut |path, source| {
        for forbidden in ["wgpu::", "glyphon::"] {
            if source.contains(forbidden) {
                violations.push(format!("{} contains `{forbidden}`", path.display()));
            }
        }
    });
    assert!(
        violations.is_empty(),
        "zeta-ui source must remain graphics-backend neutral:\n{}",
        violations.join("\n")
    );
}

#[test]
fn gpu_backend_does_not_own_interaction_or_accessibility_frames() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("native crate should be inside the Rust workspace");
    let backend_root = workspace.join("wgpu");
    let manifest = fs::read_to_string(backend_root.join("Cargo.toml"))
        .expect("zeta-wgpu manifest should be readable");
    assert!(
        !manifest
            .lines()
            .any(|line| ["zeta-ui-dispatch", "accesskit"]
                .iter()
                .any(|dependency| line.trim_start().starts_with(dependency))),
        "zeta-wgpu must consume only paint scenes; interaction and accessibility stay in the host presentation"
    );

    let mut violations = Vec::new();
    visit_rust_sources(&backend_root.join("src"), &mut |path, source| {
        for forbidden in ["zeta_ui_dispatch", "InteractionFrame", "AccessibilityNode"] {
            if source.contains(forbidden) {
                violations.push(format!("{} contains `{forbidden}`", path.display()));
            }
        }
    });
    assert!(
        violations.is_empty(),
        "GPU backend must not route input or accessibility semantics:\n{}",
        violations.join("\n")
    );
}

#[test]
fn box_owning_components_declare_inspection_metadata() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("native crate should be inside the Rust workspace");
    let mut violations = Vec::new();
    let mut implementation_count = 0;
    for crate_root in workspace_crate_roots(workspace_root) {
        let source_root = crate_root.join("src");
        visit_rust_sources(&source_root, &mut |path, source| {
            let implementations = source
                .match_indices(" Component for ")
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            for (implementation_index, start) in implementations.iter().copied().enumerate() {
                implementation_count += 1;
                let end = implementations
                    .get(implementation_index + 1)
                    .copied()
                    .unwrap_or(source.len());
                let implementation = &source[start..end];
                if implementation.contains("fn inspection(&self)") {
                    continue;
                }
                let relative = path.strip_prefix(workspace_root).unwrap_or(path);
                if relative == Path::new("ui/src/components/context_view.rs")
                    && implementation.contains("with_inspection_node")
                {
                    continue;
                }
                violations.push(relative.display().to_string());
            }
        });
    }

    assert!(
        implementation_count >= 20,
        "component audit found too few implementations"
    );
    assert!(
        violations.is_empty(),
        "Every box-owning Component must declare ComponentInspection; custom overlay components must register explicitly:\n{}",
        violations.join("\n")
    );
}

fn visit_rust_sources(root: &Path, visitor: &mut impl FnMut(&Path, &str)) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", root.display()))
        .map(|entry| entry.expect("source directory entry").path())
        .collect::<Vec<PathBuf>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            visit_rust_sources(&path, visitor);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
            visitor(&path, &source);
        }
    }
}

fn workspace_crate_roots(workspace_root: &Path) -> Vec<PathBuf> {
    let mut roots = fs::read_dir(workspace_root)
        .expect("Rust workspace directory")
        .map(|entry| entry.expect("Rust workspace entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src").is_dir())
        .collect::<Vec<_>>();
    roots.sort();
    roots
}
