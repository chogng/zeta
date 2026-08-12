use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest;
use sha2::Sha256;
use zeta_workspace::WorkspaceRoot;

use crate::ChunkContentHash;
use crate::ChunkKey;
use crate::ChunkReference;
use crate::CodeIndexError;
use crate::CodeIndexLimits;
use crate::CodeIndexManifest;
use crate::CodeIndexQuery;
use crate::CodeIndexSnapshot;
use crate::CodeIndexStorage;
use crate::IndexRootId;
use crate::IndexedChunkReference;
use crate::IndexedSourceReference;
use crate::MaterializedChunk;
use crate::MaterializedExcerpt;
use crate::MaterializedSource;
use crate::RefreshOutcome;
use crate::SearchHit;
use crate::SourceExcerptReference;
use crate::chunker::CHUNKER_VERSION;
use crate::chunker::end_line_exclusive;
use crate::chunker::language_for_path;
use crate::chunker::line_at;
use crate::chunker::line_starts;
use crate::chunker::source_revision;
use crate::error::io_error;
use crate::scanner::prepare_relative_file;
use crate::scanner::scan_workspace;
use crate::store::FileUpdate;
use crate::store::IndexStore;
use crate::store::StoredSource;

/// One workspace-side code index with a rebuildable local projection.
pub struct CodeIndex {
    root: WorkspaceRoot,
    root_id: IndexRootId,
    limits: CodeIndexLimits,
    store: IndexStore,
}

impl CodeIndex {
    /// Opens a local index projection for one already-authorized workspace root.
    ///
    /// Opening does not scan the workspace. Call [`Self::rebuild`] after watcher registration so
    /// filesystem mutations during the initial scan remain observable to the host.
    pub fn open(
        root: WorkspaceRoot,
        storage: CodeIndexStorage,
        limits: CodeIndexLimits,
    ) -> Result<Self, CodeIndexError> {
        validate_limits(&limits)?;
        let root_id = IndexRootId::from_root(&root);
        let store = IndexStore::open(&storage, &root_id)?;
        Ok(Self {
            root,
            root_id,
            limits,
            store,
        })
    }

    /// Returns the canonical workspace boundary indexed by this instance.
    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    /// Returns the stable canonical identity of the indexed workspace root.
    pub fn root_id(&self) -> &IndexRootId {
        &self.root_id
    }

    /// Returns the currently published projection summary without reading source files.
    pub fn snapshot(&self) -> Result<CodeIndexSnapshot, CodeIndexError> {
        self.store.snapshot(&self.root_id)
    }

    /// Returns the source and chunk identities published in the current atomic generation.
    pub fn manifest(&self) -> Result<CodeIndexManifest, CodeIndexError> {
        self.store.manifest(&self.root_id)
    }

    /// Performs a deterministic full scan and atomically replaces the published projection.
    pub fn rebuild(&self) -> Result<CodeIndexSnapshot, CodeIndexError> {
        let scan = scan_workspace(&self.root, &self.limits)?;
        self.store.replace_workspace(&self.root_id, scan)
    }

    /// Applies coarse watcher hints, rebuilding when a directory-level change cannot be safely
    /// represented as exact file updates.
    pub fn refresh_observed_paths(
        &self,
        observed_paths: &[PathBuf],
    ) -> Result<RefreshOutcome, CodeIndexError> {
        let mut relative_paths = observed_paths
            .iter()
            .filter_map(|path| self.root.project_observed_path(path))
            .collect::<Vec<_>>();
        relative_paths.sort_unstable();
        relative_paths.dedup();
        if relative_paths.is_empty() {
            return Ok(RefreshOutcome::NoChange);
        }

        let snapshot = self.snapshot()?;
        if snapshot.file_limit_hit || snapshot.source_bytes_limit_hit {
            return self.rebuild().map(RefreshOutcome::Rebuilt);
        }

        let mut updates = Vec::new();
        for relative_path in relative_paths {
            if relative_path.as_os_str().is_empty() || is_ignore_control_path(&relative_path) {
                return self.rebuild().map(RefreshOutcome::Rebuilt);
            }
            if is_hard_excluded_path(&relative_path) {
                continue;
            }
            let candidate = self.root.canonical_path().join(&relative_path);
            if candidate.is_dir() {
                return self.rebuild().map(RefreshOutcome::Rebuilt);
            }
            if candidate.is_file() {
                let Some(stored) = self.store.source(&relative_path)? else {
                    return self.rebuild().map(RefreshOutcome::Rebuilt);
                };
                let Some(file) =
                    prepare_relative_file(&self.root, relative_path.clone(), &self.limits)?
                else {
                    return self.rebuild().map(RefreshOutcome::Rebuilt);
                };
                if stored.revision == file.source_revision {
                    continue;
                }
                if exceeds_source_limit(&snapshot, &stored, file.source_bytes, &self.limits) {
                    return self.rebuild().map(RefreshOutcome::Rebuilt);
                }
                updates.push(FileUpdate::Upsert(file));
                continue;
            }
            if self.store.source(&relative_path)?.is_some() {
                updates.push(FileUpdate::Remove(relative_path));
            } else if self.store.has_descendants(&relative_path)? {
                return self.rebuild().map(RefreshOutcome::Rebuilt);
            }
        }
        if updates.is_empty() {
            return Ok(RefreshOutcome::NoChange);
        }
        self.store
            .publish_updates(&self.root_id, updates)
            .map(RefreshOutcome::Published)
    }

    /// Searches indexed path and chunk text using a literal FTS query.
    pub fn search(&self, query: &CodeIndexQuery) -> Result<Vec<SearchHit>, CodeIndexError> {
        let expression = literal_fts_expression(query.text(), self.limits.max_query_bytes)?;
        let result_limit = query.result_limit().get().min(self.limits.max_results);
        self.store.search(&self.root_id, &expression, result_limit)
    }

    /// Rereads one search result and proves that its revision, range, and chunk identities still
    /// match the current workspace file before the content is used as Agent context.
    pub fn materialize(
        &self,
        reference: &ChunkReference,
    ) -> Result<MaterializedChunk, CodeIndexError> {
        if reference.root_id != self.root_id {
            return Err(CodeIndexError::StorageRootMismatch);
        }
        let absolute_path = self.root.resolve_existing(&reference.relative_path)?;
        let source = read_source_for_verification(&absolute_path, self.limits.max_file_bytes)?;
        materialize_chunk_from_source(reference, &source)
    }

    /// Rereads one revision-bound excerpt and verifies its current source revision, byte range,
    /// and content hash without requiring the local chunker to have produced the boundary.
    pub fn materialize_excerpt(
        &self,
        reference: &SourceExcerptReference,
    ) -> Result<MaterializedExcerpt, CodeIndexError> {
        self.validate_root(&reference.root_id)?;
        let absolute_path = self.root.resolve_existing(&reference.relative_path)?;
        let source = read_source_for_verification(&absolute_path, self.limits.max_file_bytes)?;
        validate_source_revision(&source, &reference.source_revision)?;
        let content = verified_span_content(&source, &reference.span)?;
        let content_hash = ChunkContentHash::new(sha256(content.as_bytes()));
        if content_hash != reference.content_hash {
            return Err(CodeIndexError::ChunkIdentityMismatch);
        }
        Ok(MaterializedExcerpt {
            reference: reference.clone(),
            language: language_for_path(Path::new(&reference.relative_path)),
            content: content.to_owned(),
        })
    }

    /// Rereads and verifies complete source files selected from one published manifest.
    pub fn materialize_sources(
        &self,
        references: &[IndexedSourceReference],
    ) -> Result<Vec<MaterializedSource>, CodeIndexError> {
        references
            .iter()
            .map(|reference| {
                self.validate_root(&reference.root_id)?;
                let absolute_path = self.root.resolve_existing(&reference.relative_path)?;
                let content =
                    read_source_for_verification(&absolute_path, self.limits.max_file_bytes)?;
                validate_source_revision(&content, &reference.source_revision)?;
                Ok(MaterializedSource {
                    reference: reference.clone(),
                    content,
                })
            })
            .collect()
    }

    /// Rereads each selected source at most once and verifies all selected chunk identities.
    pub fn materialize_chunks(
        &self,
        references: &[IndexedChunkReference],
    ) -> Result<Vec<MaterializedChunk>, CodeIndexError> {
        let mut by_path = BTreeMap::<PathBuf, Vec<&IndexedChunkReference>>::new();
        for reference in references {
            self.validate_root(&reference.reference.root_id)?;
            by_path
                .entry(reference.reference.relative_path.clone())
                .or_default()
                .push(reference);
        }
        let mut materialized = Vec::with_capacity(references.len());
        for (relative_path, path_references) in by_path {
            let absolute_path = self.root.resolve_existing(&relative_path)?;
            let source = read_source_for_verification(&absolute_path, self.limits.max_file_bytes)?;
            for reference in path_references {
                let mut chunk = materialize_chunk_from_source(&reference.reference, &source)?;
                chunk.language = reference.language;
                materialized.push(chunk);
            }
        }
        Ok(materialized)
    }

    fn validate_root(&self, root_id: &IndexRootId) -> Result<(), CodeIndexError> {
        if root_id == &self.root_id {
            Ok(())
        } else {
            Err(CodeIndexError::StorageRootMismatch)
        }
    }
}

fn materialize_chunk_from_source(
    reference: &ChunkReference,
    source: &str,
) -> Result<MaterializedChunk, CodeIndexError> {
    validate_source_revision(source, &reference.source_revision)?;
    let content = verified_span_content(source, &reference.span)?;
    let content_hash = ChunkContentHash::new(sha256(content.as_bytes()));
    let key = ChunkKey::new(sha256(
        [CHUNKER_VERSION.as_bytes(), b"\0", content.as_bytes()].concat(),
    ));
    if content_hash != reference.content_hash || key != reference.key {
        return Err(CodeIndexError::ChunkIdentityMismatch);
    }
    Ok(MaterializedChunk {
        reference: reference.clone(),
        language: language_for_path(Path::new(&reference.relative_path)),
        content: content.to_owned(),
    })
}

fn verified_span_content<'a>(
    source: &'a str,
    span: &crate::ChunkSpan,
) -> Result<&'a str, CodeIndexError> {
    if span.start_byte >= span.end_byte || span.start_line >= span.end_line_exclusive {
        return Err(CodeIndexError::InvalidChunkRange);
    }
    let Some(content) = source.get(span.byte_range()) else {
        return Err(CodeIndexError::InvalidChunkRange);
    };
    let starts = line_starts(source);
    let start_line = line_at(&starts, span.start_byte);
    let end_line = end_line_exclusive(&starts, span.end_byte, source.len());
    if span.start_line != start_line || span.end_line_exclusive != end_line {
        return Err(CodeIndexError::ChunkSpanMismatch);
    }
    Ok(content)
}

fn validate_source_revision(
    source: &str,
    expected: &crate::SourceRevision,
) -> Result<(), CodeIndexError> {
    let observed = source_revision(source);
    if &observed == expected {
        Ok(())
    } else {
        Err(CodeIndexError::StaleRevision {
            expected: expected.clone(),
            observed,
        })
    }
}

fn read_source_for_verification(
    path: &Path,
    max_file_bytes: usize,
) -> Result<String, CodeIndexError> {
    let file = fs::File::open(path).map_err(|source| io_error(path, source))?;
    let read_limit = u64::try_from(max_file_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::with_capacity(max_file_bytes.min(64 * 1024));
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error(path, source))?;
    if bytes.len() > max_file_bytes {
        return Err(CodeIndexError::SourceVerificationLimitExceeded);
    }
    String::from_utf8(bytes).map_err(|source| {
        io_error(
            path,
            std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        )
    })
}

fn exceeds_source_limit(
    snapshot: &CodeIndexSnapshot,
    stored: &StoredSource,
    replacement_bytes: usize,
    limits: &CodeIndexLimits,
) -> bool {
    snapshot
        .indexed_source_bytes
        .saturating_sub(stored.source_bytes)
        .saturating_add(replacement_bytes)
        > limits.max_total_source_bytes
}

fn is_ignore_control_path(relative_path: &Path) -> bool {
    relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".gitignore" | ".ignore" | ".gitmodules"))
        || relative_path == Path::new(".git/info/exclude")
}

fn is_hard_excluded_path(relative_path: &Path) -> bool {
    relative_path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| matches!(name, ".git" | ".zeta" | "node_modules" | "target"))
    })
}

fn validate_limits(limits: &CodeIndexLimits) -> Result<(), CodeIndexError> {
    if limits.target_chunk_bytes > limits.max_chunk_bytes {
        return Err(CodeIndexError::InvalidLimits(
            "target chunk size must not exceed the hard chunk size",
        ));
    }
    if limits.max_chunk_bytes > limits.max_file_bytes {
        return Err(CodeIndexError::InvalidLimits(
            "hard chunk size must not exceed the maximum file size",
        ));
    }
    if limits.max_file_bytes > limits.max_total_source_bytes {
        return Err(CodeIndexError::InvalidLimits(
            "maximum file size must not exceed the total source-byte limit",
        ));
    }
    Ok(())
}

fn literal_fts_expression(text: &str, max_query_bytes: usize) -> Result<String, CodeIndexError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(CodeIndexError::InvalidQuery("query must not be empty"));
    }
    if text.len() > max_query_bytes {
        return Err(CodeIndexError::InvalidQuery("query exceeds the byte limit"));
    }
    let terms = text
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Err(CodeIndexError::InvalidQuery("query must contain text"));
    }
    Ok(terms.join(" AND "))
}

fn sha256(bytes: impl AsRef<[u8]>) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes.as_ref());
    format!("sha256:{:x}", digest.finalize())
}
