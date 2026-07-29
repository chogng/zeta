use crate::{SetiFileIconAssociations, SetiFileIconManifest, SetiIconDefinition};
use std::collections::BTreeMap;

/// Color-specific association set selected by a renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetiColorScheme {
    Dark,
    Light,
}

/// Icon identity and browser artwork selected for one file name.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedSetiFileIcon<'a> {
    pub icon_id: &'a str,
    pub definition: &'a SetiIconDefinition,
}

/// Resolves a basename using exact-name, longest-extension, language, and default precedence.
pub fn resolve_file_icon<'a>(
    manifest: &'a SetiFileIconManifest,
    file_name: &str,
    color_scheme: SetiColorScheme,
) -> Option<ResolvedSetiFileIcon<'a>> {
    let normalized = file_name.to_lowercase();
    let icon_id = match color_scheme {
        SetiColorScheme::Dark => resolve_specific(
            &manifest.file_names,
            &manifest.file_extensions,
            &manifest.language_ids,
            &normalized,
        )
        .unwrap_or(&manifest.file),
        SetiColorScheme::Light => resolve_specific_associations(&manifest.light, &normalized)
            .or_else(|| {
                resolve_specific(
                    &manifest.file_names,
                    &manifest.file_extensions,
                    &manifest.language_ids,
                    &normalized,
                )
            })
            .unwrap_or(&manifest.light.file),
    };
    manifest
        .icon_definitions
        .get(icon_id)
        .map(|definition| ResolvedSetiFileIcon {
            icon_id,
            definition,
        })
}

fn resolve_specific_associations<'a>(
    associations: &'a SetiFileIconAssociations,
    file_name: &str,
) -> Option<&'a str> {
    resolve_specific(
        &associations.file_names,
        &associations.file_extensions,
        &associations.language_ids,
        file_name,
    )
}

fn resolve_specific<'a>(
    file_names: &'a BTreeMap<String, String>,
    file_extensions: &'a BTreeMap<String, String>,
    language_ids: &'a BTreeMap<String, String>,
    file_name: &str,
) -> Option<&'a str> {
    if let Some(icon_id) = file_names.get(file_name) {
        return Some(icon_id);
    }
    for extension in extension_candidates(file_name) {
        if let Some(icon_id) = file_extensions.get(extension.as_str()) {
            return Some(icon_id);
        }
    }
    let extension = file_name.rsplit('.').next().unwrap_or(file_name);
    let language_id = language_id_for_extension(extension, language_ids)?;
    language_ids.get(language_id).map(String::as_str)
}

fn extension_candidates(file_name: &str) -> Vec<String> {
    let segments = file_name.split('.').collect::<Vec<_>>();
    if segments.len() < 2 {
        return vec![file_name.into()];
    }
    (1..segments.len())
        .map(|index| segments[index..].join("."))
        .collect()
}

fn language_id_for_extension<'a>(
    extension: &'a str,
    language_ids: &BTreeMap<String, String>,
) -> Option<&'a str> {
    let mapped = match extension {
        "bash" | "sh" | "zsh" => "shellscript",
        "cc" | "cxx" | "hh" | "hpp" => "cpp",
        "cjs" | "js" | "mjs" => "javascript",
        "clj" | "cljs" => "clojure",
        "coffee" => "coffeescript",
        "cs" => "csharp",
        "fs" | "fsx" => "fsharp",
        "h" => "c",
        "hs" => "haskell",
        "jsx" => "javascriptreact",
        "kt" | "kts" => "kotlin",
        "md" => "markdown",
        "pl" | "pm" => "perl",
        "ps1" => "powershell",
        "py" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "yml" => "yaml",
        _ if language_ids.contains_key(extension) => extension,
        _ => return None,
    };
    Some(mapped)
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
