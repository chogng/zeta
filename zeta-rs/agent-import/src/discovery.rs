use std::fs;
use std::io;

use crate::layout::{CandidateSpec, ExpectedPathKind, candidate_specs};
use crate::{
    AgentImportCandidate, AgentImportDiagnostic, AgentImportDiagnosticCode, AgentImportError,
    AgentImportLocation, AgentImportPlan,
};

pub(super) fn discover(
    locations: impl IntoIterator<Item = AgentImportLocation>,
) -> Result<AgentImportPlan, AgentImportError> {
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for location in locations {
        let canonical_root = validate_root(&location)?;
        for spec in candidate_specs(location.agent(), location.scope()) {
            inspect_candidate(
                &location,
                &canonical_root,
                spec,
                &mut candidates,
                &mut diagnostics,
            );
        }
    }
    candidates.sort_by(|left, right| {
        (
            left.agent(),
            left.scope(),
            left.kind(),
            left.relative_path(),
        )
            .cmp(&(
                right.agent(),
                right.scope(),
                right.kind(),
                right.relative_path(),
            ))
    });
    candidates.dedup_by(|left, right| {
        left.agent() == right.agent()
            && left.scope() == right.scope()
            && left.kind() == right.kind()
            && left.source_path() == right.source_path()
    });
    diagnostics.sort_by(|left, right| {
        (
            left.agent(),
            left.scope(),
            left.kind(),
            left.relative_path(),
            left.code(),
        )
            .cmp(&(
                right.agent(),
                right.scope(),
                right.kind(),
                right.relative_path(),
                right.code(),
            ))
    });
    diagnostics.dedup();
    Ok(AgentImportPlan::new(candidates, diagnostics))
}

fn validate_root(location: &AgentImportLocation) -> Result<std::path::PathBuf, AgentImportError> {
    let metadata = fs::symlink_metadata(location.root()).map_err(|error| {
        AgentImportError::RootUnavailable {
            agent: location.agent(),
            scope: location.scope(),
            error_kind: error.kind(),
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(AgentImportError::RootSymlinkNotAllowed {
            agent: location.agent(),
            scope: location.scope(),
        });
    }
    if !metadata.is_dir() {
        return Err(AgentImportError::RootNotDirectory {
            agent: location.agent(),
            scope: location.scope(),
        });
    }
    fs::canonicalize(location.root()).map_err(|error| AgentImportError::RootUnavailable {
        agent: location.agent(),
        scope: location.scope(),
        error_kind: error.kind(),
    })
}

fn inspect_candidate(
    location: &AgentImportLocation,
    canonical_root: &std::path::Path,
    spec: &CandidateSpec,
    candidates: &mut Vec<AgentImportCandidate>,
    diagnostics: &mut Vec<AgentImportDiagnostic>,
) {
    let relative_path = std::path::PathBuf::from(spec.relative_path);
    let candidate = canonical_root.join(&relative_path);
    let metadata = match fs::symlink_metadata(&candidate) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(_) => {
            diagnostics.push(AgentImportDiagnostic::new(
                location,
                spec.kind,
                relative_path,
                AgentImportDiagnosticCode::MetadataUnavailable,
            ));
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        diagnostics.push(AgentImportDiagnostic::new(
            location,
            spec.kind,
            relative_path,
            AgentImportDiagnosticCode::SymlinkNotAllowed,
        ));
        return;
    }
    let expected_type_matches = match spec.expected {
        ExpectedPathKind::File => metadata.is_file(),
        ExpectedPathKind::Directory => metadata.is_dir(),
    };
    if !expected_type_matches {
        diagnostics.push(AgentImportDiagnostic::new(
            location,
            spec.kind,
            relative_path,
            AgentImportDiagnosticCode::UnexpectedFileType,
        ));
        return;
    }
    let canonical_candidate = match fs::canonicalize(&candidate) {
        Ok(path) => path,
        Err(_) => {
            diagnostics.push(AgentImportDiagnostic::new(
                location,
                spec.kind,
                relative_path,
                AgentImportDiagnosticCode::MetadataUnavailable,
            ));
            return;
        }
    };
    if !canonical_candidate.starts_with(canonical_root) {
        diagnostics.push(AgentImportDiagnostic::new(
            location,
            spec.kind,
            relative_path,
            AgentImportDiagnosticCode::EscapesSelectedRoot,
        ));
        return;
    }
    candidates.push(AgentImportCandidate::new(
        location,
        spec.kind,
        spec.review,
        relative_path,
        canonical_candidate,
    ));
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
