use crate::FastRegexCaseSensitivity;
use crate::FastRegexError;
use crate::FastRegexMatch;
use crate::FastRegexPattern;
use crate::FastRegexQuery;
use crate::FastRegexRange;
use crate::FastRegexSearchLimits;
use crate::FastRegexSearchResult;
use crate::FastRegexSearchSnapshot;
use crate::FastRegexSearchStorage;
use crate::FastRegexUpdateOutcome;
use crate::ngram::sparse_ngrams;
use globset::GlobBuilder;
use globset::GlobSet;
use globset::GlobSetBuilder;
use ignore::WalkBuilder;
use regex::Regex;
use regex::RegexBuilder;
use regex_syntax::hir::literal::ExtractKind;
use regex_syntax::hir::literal::Extractor;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
use zeta_workspace::WorkspaceRoot;

const STORE_VERSION: &[u8] = b"zeta-fast-regex-v1\0";

pub struct FastRegexSearch {
    root: WorkspaceRoot,
    storage: FastRegexSearchStorage,
    limits: FastRegexSearchLimits,
    state: RwLock<IndexState>,
}

#[derive(Default)]
struct IndexState {
    generation: u64,
    source_bytes: usize,
    documents: BTreeMap<PathBuf, IndexedDocument>,
    postings: HashMap<u64, BTreeSet<PathBuf>>,
    folded_postings: HashMap<u64, BTreeSet<PathBuf>>,
    overlays: BTreeMap<PathBuf, String>,
}

struct IndexedDocument {
    revision: String,
    source_bytes: usize,
    grams: Vec<u64>,
    folded_grams: Vec<u64>,
}

impl FastRegexSearch {
    pub fn open(
        root: WorkspaceRoot,
        storage: FastRegexSearchStorage,
        limits: FastRegexSearchLimits,
    ) -> Result<Self, FastRegexError> {
        if limits.max_files == 0
            || limits.max_file_bytes == 0
            || limits.max_total_source_bytes < limits.max_file_bytes
            || limits.max_query_bytes == 0
            || limits.max_results == 0
        {
            return Err(FastRegexError::InvalidLimits);
        }
        if let FastRegexSearchStorage::Persistent(path) = &storage {
            fs::create_dir_all(path).map_err(|source| io_error(path, source))?;
        }
        Ok(Self {
            root,
            storage,
            limits,
            state: RwLock::new(IndexState::default()),
        })
    }

    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub fn snapshot(&self) -> FastRegexSearchSnapshot {
        snapshot(&self.state.read().unwrap_or_else(|error| error.into_inner()))
    }

    pub fn rebuild(&self) -> Result<FastRegexSearchSnapshot, FastRegexError> {
        let prepared = scan_workspace(&self.root, &self.limits)?;
        let mut next = IndexState::default();
        for (path, content) in prepared {
            insert_document(&mut next, path, content);
        }
        let mut state = self
            .state
            .write()
            .unwrap_or_else(|error| error.into_inner());
        next.generation = state.generation.saturating_add(1);
        next.overlays = std::mem::take(&mut state.overlays);
        persist(&self.storage, &next)?;
        *state = next;
        Ok(snapshot(&state))
    }

    pub fn refresh_observed_paths(
        &self,
        observed_paths: &[PathBuf],
    ) -> Result<FastRegexUpdateOutcome, FastRegexError> {
        let mut paths = observed_paths
            .iter()
            .filter_map(|path| self.root.project_observed_path(path))
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        if paths.is_empty() {
            return Ok(FastRegexUpdateOutcome::NoChange);
        }
        if paths.iter().any(|path| {
            path.as_os_str().is_empty()
                || is_ignore_control(path)
                || self.root.canonical_path().join(path).is_dir()
        }) {
            return self.rebuild().map(FastRegexUpdateOutcome::Rebuilt);
        }
        let mut state = self
            .state
            .write()
            .unwrap_or_else(|error| error.into_inner());
        let mut changed = false;
        for path in paths {
            if is_hard_excluded(&path) {
                continue;
            }
            let absolute = self.root.canonical_path().join(&path);
            if !absolute.is_file() {
                changed |= remove_document(&mut state, &path);
                continue;
            }
            if !state.documents.contains_key(&path) {
                drop(state);
                return self.rebuild().map(FastRegexUpdateOutcome::Rebuilt);
            }
            match read_text_file(&absolute, self.limits.max_file_bytes)? {
                Some(content) => {
                    let current_revision = revision(&content);
                    if state
                        .documents
                        .get(&path)
                        .is_some_and(|document| document.revision == current_revision)
                    {
                        continue;
                    }
                    remove_document(&mut state, &path);
                    insert_document(&mut state, path, content);
                    changed = true;
                }
                None => changed |= remove_document(&mut state, &path),
            }
        }
        if !changed {
            return Ok(FastRegexUpdateOutcome::NoChange);
        }
        state.generation = state.generation.saturating_add(1);
        persist(&self.storage, &state)?;
        Ok(FastRegexUpdateOutcome::Published(snapshot(&state)))
    }

    pub fn synchronize_overlay(
        &self,
        path: PathBuf,
        content: String,
    ) -> Result<(), FastRegexError> {
        validate_relative_path(&path)?;
        if content.len() > self.limits.max_file_bytes {
            return Err(FastRegexError::InvalidQuery(
                "overlay exceeds the file byte limit",
            ));
        }
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .overlays
            .insert(path, content);
        Ok(())
    }

    pub fn close_overlay(&self, path: &Path) -> Result<(), FastRegexError> {
        validate_relative_path(path)?;
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .overlays
            .remove(path);
        Ok(())
    }

    pub fn search(&self, query: &FastRegexQuery) -> Result<FastRegexSearchResult, FastRegexError> {
        validate_query(query, &self.limits)?;
        let sensitive = case_sensitive(query);
        let matcher = compile_matcher(query, sensitive)?;
        let filters = PathFilters::new(&query.include_patterns, &query.exclude_patterns)?;
        let state = self.state.read().unwrap_or_else(|error| error.into_inner());
        if state.generation == 0 {
            return Err(FastRegexError::NotReady);
        }
        let candidates = candidate_paths(&state, query, sensitive);
        let mut matches = Vec::new();
        let mut limit_hit = false;
        for path in candidates {
            if !filters.matches(&path) {
                continue;
            }
            let content = if let Some(content) = state.overlays.get(&path) {
                content.clone()
            } else {
                let Some(document) = state.documents.get(&path) else {
                    continue;
                };
                let absolute = self.root.canonical_path().join(&path);
                let Some(content) = read_text_file(&absolute, self.limits.max_file_bytes)? else {
                    return Err(FastRegexError::StaleSource(path));
                };
                if revision(&content) != document.revision {
                    return Err(FastRegexError::StaleSource(path));
                }
                content
            };
            collect_matches(
                &path,
                &content,
                &matcher,
                query.max_results,
                &mut matches,
                &mut limit_hit,
            );
            if limit_hit {
                break;
            }
        }
        Ok(FastRegexSearchResult { matches, limit_hit })
    }
}

fn insert_document(state: &mut IndexState, path: PathBuf, content: String) {
    let grams = sparse_ngrams(content.as_bytes());
    let folded = content
        .as_bytes()
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let folded_grams = sparse_ngrams(&folded);
    for gram in &grams {
        state
            .postings
            .entry(*gram)
            .or_default()
            .insert(path.clone());
    }
    for gram in &folded_grams {
        state
            .folded_postings
            .entry(*gram)
            .or_default()
            .insert(path.clone());
    }
    state.source_bytes = state.source_bytes.saturating_add(content.len());
    state.documents.insert(
        path,
        IndexedDocument {
            revision: revision(&content),
            source_bytes: content.len(),
            grams,
            folded_grams,
        },
    );
}

fn remove_document(state: &mut IndexState, path: &Path) -> bool {
    let Some(document) = state.documents.remove(path) else {
        return false;
    };
    state.source_bytes = state.source_bytes.saturating_sub(document.source_bytes);
    remove_postings(&mut state.postings, path, &document.grams);
    remove_postings(&mut state.folded_postings, path, &document.folded_grams);
    true
}

fn remove_postings(postings: &mut HashMap<u64, BTreeSet<PathBuf>>, path: &Path, grams: &[u64]) {
    for gram in grams {
        if let Some(paths) = postings.get_mut(gram) {
            paths.remove(path);
            if paths.is_empty() {
                postings.remove(gram);
            }
        }
    }
}

fn candidate_paths(
    state: &IndexState,
    query: &FastRegexQuery,
    sensitive: bool,
) -> BTreeSet<PathBuf> {
    let literals = required_literals(query, sensitive);
    let postings = if sensitive {
        &state.postings
    } else {
        &state.folded_postings
    };
    let mut candidates = match literals.as_ref() {
        None => state.documents.keys().cloned().collect(),
        Some(literals) if literals.is_empty() => BTreeSet::new(),
        Some(literals) => {
            let mut union = BTreeSet::new();
            for literal in literals {
                let grams = sparse_ngrams(literal);
                if grams.is_empty() {
                    return state.documents.keys().cloned().collect();
                }
                if let Some(paths) = grams
                    .iter()
                    .filter_map(|gram| postings.get(gram))
                    .min_by_key(|paths| paths.len())
                {
                    union.extend(paths.iter().cloned());
                }
            }
            union
        }
    };
    for (path, content) in &state.overlays {
        candidates.remove(path);
        let bytes = if sensitive {
            content.as_bytes().to_vec()
        } else {
            content
                .as_bytes()
                .iter()
                .map(u8::to_ascii_lowercase)
                .collect()
        };
        let include = literals
            .as_ref()
            .is_none_or(|literals| literals.iter().any(|literal| find_bytes(&bytes, literal)));
        if include {
            candidates.insert(path.clone());
        }
    }
    candidates
}

fn required_literals(query: &FastRegexQuery, sensitive: bool) -> Option<Vec<Vec<u8>>> {
    let mut literals = match query.pattern {
        FastRegexPattern::Literal => vec![query.query.as_bytes().to_vec()],
        FastRegexPattern::Regex => {
            let hir = regex_syntax::parse(&query.query).ok()?;
            let prefix = Extractor::new().extract(&hir);
            let mut suffix_extractor = Extractor::new();
            suffix_extractor.kind(ExtractKind::Suffix);
            let suffix = suffix_extractor.extract(&hir);
            let best = [prefix, suffix]
                .into_iter()
                .filter_map(|sequence| sequence.literals().map(<[_]>::to_vec))
                .filter(|literals| !literals.iter().any(|literal| literal.as_bytes().is_empty()))
                .max_by_key(|literals| {
                    literals
                        .iter()
                        .map(|literal| literal.len())
                        .min()
                        .unwrap_or(0)
                })?;
            best.into_iter()
                .map(|literal| literal.as_bytes().to_vec())
                .collect()
        }
    };
    if !sensitive {
        if literals.iter().any(|literal| !literal.is_ascii()) {
            return None;
        }
        for literal in &mut literals {
            literal.make_ascii_lowercase();
        }
    }
    Some(literals)
}

fn compile_matcher(query: &FastRegexQuery, sensitive: bool) -> Result<Regex, FastRegexError> {
    let expression = match query.pattern {
        FastRegexPattern::Literal => regex::escape(&query.query),
        FastRegexPattern::Regex => query.query.clone(),
    };
    RegexBuilder::new(&expression)
        .case_insensitive(!sensitive)
        .multi_line(false)
        .build()
        .map_err(FastRegexError::from)
}

fn case_sensitive(query: &FastRegexQuery) -> bool {
    match query.case_sensitivity {
        FastRegexCaseSensitivity::Sensitive => true,
        FastRegexCaseSensitivity::Insensitive => false,
        FastRegexCaseSensitivity::Smart => query.query.chars().any(char::is_uppercase),
    }
}

fn collect_matches(
    path: &Path,
    content: &str,
    matcher: &Regex,
    limit: usize,
    output: &mut Vec<FastRegexMatch>,
    limit_hit: &mut bool,
) {
    for (line_index, raw_line) in content.split('\n').enumerate() {
        let preview = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let ranges = matcher
            .find_iter(preview)
            .map(|found| FastRegexRange {
                start_byte: found.start(),
                end_byte: found.end(),
            })
            .collect::<Vec<_>>();
        if ranges.is_empty() {
            continue;
        }
        if output.len() == limit {
            *limit_hit = true;
            return;
        }
        output.push(FastRegexMatch {
            path: path.to_path_buf(),
            line_number: line_index + 1,
            preview: preview.to_owned(),
            ranges,
        });
    }
}

struct PathFilters {
    includes: Option<GlobSet>,
    excludes: GlobSet,
}

impl PathFilters {
    fn new(includes: &[String], excludes: &[String]) -> Result<Self, FastRegexError> {
        Ok(Self {
            includes: (!includes.is_empty())
                .then(|| build_glob_set(includes))
                .transpose()?,
            excludes: build_glob_set(excludes)?,
        })
    }

    fn matches(&self, path: &Path) -> bool {
        self.includes.as_ref().is_none_or(|set| set.is_match(path)) && !self.excludes.is_match(path)
    }
}

fn build_glob_set(patterns: &[String]) -> Result<GlobSet, FastRegexError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map_err(|_| FastRegexError::InvalidGlob)?;
        builder.add(glob);
    }
    builder.build().map_err(|_| FastRegexError::InvalidGlob)
}

fn scan_workspace(
    root: &WorkspaceRoot,
    limits: &FastRegexSearchLimits,
) -> Result<Vec<(PathBuf, String)>, FastRegexError> {
    let mut paths = WalkBuilder::new(root.canonical_path())
        .hidden(true)
        .follow_links(false)
        .require_git(true)
        .filter_entry(|entry| entry.depth() == 0 || !is_hard_excluded(entry.path()))
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(root.canonical_path())
                .ok()
                .map(Path::to_path_buf)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.truncate(limits.max_files);
    let mut source_bytes = 0usize;
    let mut documents = Vec::new();
    for path in paths {
        let absolute = root.canonical_path().join(&path);
        let Some(content) = read_text_file(&absolute, limits.max_file_bytes)? else {
            continue;
        };
        if source_bytes.saturating_add(content.len()) > limits.max_total_source_bytes {
            break;
        }
        source_bytes = source_bytes.saturating_add(content.len());
        documents.push((path, content));
    }
    Ok(documents)
}

fn read_text_file(path: &Path, max_bytes: usize) -> Result<Option<String>, FastRegexError> {
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    if bytes.len() > max_bytes || bytes.iter().take(8 * 1024).any(|byte| *byte == 0) {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}

fn persist(storage: &FastRegexSearchStorage, state: &IndexState) -> Result<(), FastRegexError> {
    let FastRegexSearchStorage::Persistent(directory) = storage else {
        return Ok(());
    };
    let mut ids = BTreeMap::new();
    let mut documents = Vec::new();
    documents.extend_from_slice(STORE_VERSION);
    documents.extend_from_slice(&(state.documents.len() as u64).to_le_bytes());
    for (id, (path, document)) in state.documents.iter().enumerate() {
        ids.insert(path, id as u32);
        let path = path.to_string_lossy();
        documents.extend_from_slice(&(path.len() as u32).to_le_bytes());
        documents.extend_from_slice(path.as_bytes());
        documents.extend_from_slice(&(document.source_bytes as u64).to_le_bytes());
        documents.extend_from_slice(&(document.revision.len() as u32).to_le_bytes());
        documents.extend_from_slice(document.revision.as_bytes());
    }
    let mut postings_bytes = Vec::new();
    let mut lookup_bytes = Vec::new();
    lookup_bytes.extend_from_slice(STORE_VERSION);
    let mut entries = state.postings.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(gram, _)| **gram);
    lookup_bytes.extend_from_slice(&(entries.len() as u64).to_le_bytes());
    for (gram, paths) in entries {
        let offset = postings_bytes.len() as u64;
        postings_bytes.extend_from_slice(&(paths.len() as u32).to_le_bytes());
        for path in paths {
            postings_bytes.extend_from_slice(&ids[path].to_le_bytes());
        }
        lookup_bytes.extend_from_slice(&gram.to_le_bytes());
        lookup_bytes.extend_from_slice(&offset.to_le_bytes());
        lookup_bytes.extend_from_slice(&(paths.len() as u32).to_le_bytes());
    }
    write_atomic(directory.join("documents.bin"), &documents)?;
    write_atomic(directory.join("postings.bin"), &postings_bytes)?;
    write_atomic(directory.join("lookup.bin"), &lookup_bytes)
}

fn write_atomic(path: PathBuf, bytes: &[u8]) -> Result<(), FastRegexError> {
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|source| io_error(&temporary, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(&temporary, source))?;
    file.sync_all()
        .map_err(|source| io_error(&temporary, source))?;
    fs::rename(&temporary, &path).map_err(|source| io_error(&path, source))
}

fn validate_query(
    query: &FastRegexQuery,
    limits: &FastRegexSearchLimits,
) -> Result<(), FastRegexError> {
    if query.query.is_empty()
        || query.query.len() > limits.max_query_bytes
        || query.query.contains('\0')
    {
        return Err(FastRegexError::InvalidQuery("search query is invalid"));
    }
    if query.max_results == 0 || query.max_results > limits.max_results {
        return Err(FastRegexError::InvalidQuery(
            "search result limit is invalid",
        ));
    }
    PathFilters::new(&query.include_patterns, &query.exclude_patterns)?;
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), FastRegexError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(FastRegexError::InvalidQuery("overlay path is invalid"));
    }
    Ok(())
}

fn is_ignore_control(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".gitignore" | ".ignore" | ".gitmodules"))
}

fn is_hard_excluded(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| matches!(name, ".git" | ".zeta" | "node_modules" | "target"))
    })
}

fn revision(content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(content.as_bytes());
    format!("sha256:{:x}", digest.finalize())
}

fn snapshot(state: &IndexState) -> FastRegexSearchSnapshot {
    FastRegexSearchSnapshot {
        generation: state.generation,
        indexed_file_count: state.documents.len(),
        indexed_source_bytes: state.source_bytes,
        indexed_ngram_count: state.postings.len(),
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || (needle.len() <= haystack.len()
            && haystack
                .windows(needle.len())
                .any(|window| window == needle))
}

fn io_error(path: &Path, source: std::io::Error) -> FastRegexError {
    FastRegexError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
