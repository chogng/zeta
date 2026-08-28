use crate::FastRegexError;
use crate::FastRegexSearchStorage;
use crate::index::IndexState;
use crate::index::IndexedDocument;
use crate::ngram::PairWeights;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

const STORE_VERSION: &[u8] = b"zeta-fast-regex-v2\0";
const COMPLETE_FILE: &str = "complete.bin";
const DELTA_FILE: &str = "delta.bin";
const DOCUMENTS_FILE: &str = "documents.bin";
const LOOKUP_FILE: &str = "lookup.bin";
const POSTINGS_FILE: &str = "postings.bin";
const WEIGHTS_FILE: &str = "weights.bin";

pub(crate) struct DiskBaseIndex {
    lookup: Vec<u8>,
    postings: Vec<u8>,
    paths: Vec<PathBuf>,
}

impl DiskBaseIndex {
    fn open(
        lookup_path: &Path,
        postings_path: &Path,
        generation: u64,
        paths: Vec<PathBuf>,
    ) -> Result<Self, FastRegexError> {
        let lookup = read_complete_generation_file(lookup_path, generation)?;
        let postings = read_complete_generation_file(postings_path, generation)?;
        validate_disk_postings(lookup_path, &lookup, &postings, paths.len())?;
        Ok(Self {
            lookup,
            postings,
            paths,
        })
    }

    pub(crate) fn intersect_postings(
        &self,
        grams: &[u64],
        folded: bool,
        excluded_paths: &BTreeSet<PathBuf>,
    ) -> BTreeSet<PathBuf> {
        let mut spans = Vec::with_capacity(grams.len());
        for gram in grams {
            let Some(span) = self.posting_span(*gram, folded) else {
                return BTreeSet::new();
            };
            spans.push(span);
        }
        spans.sort_by_key(|(_, count)| *count);
        let Some((first_start, first_count)) = spans.first().copied() else {
            return BTreeSet::new();
        };
        let mut ids = (0..first_count)
            .filter_map(|index| read_u32(&self.postings, first_start + index * 4))
            .collect::<Vec<_>>();
        for (start, count) in spans.into_iter().skip(1) {
            ids.retain(|id| posting_contains(&self.postings, start, count, *id));
            if ids.is_empty() {
                break;
            }
        }
        ids.into_iter()
            .filter_map(|id| self.paths.get(id as usize))
            .filter(|path| !excluded_paths.contains(*path))
            .cloned()
            .collect()
    }

    fn posting_span(&self, gram: u64, folded: bool) -> Option<(usize, usize)> {
        let entry_count = read_u64(&self.lookup, header_length())? as usize;
        let entries_offset = header_length() + 8;
        let target = (u8::from(folded), gram);
        let mut left = 0usize;
        let mut right = entry_count;
        while left < right {
            let middle = left + (right - left) / 2;
            let offset = entries_offset + middle * 21;
            let key = (self.lookup[offset], read_u64(&self.lookup, offset + 1)?);
            match key.cmp(&target) {
                std::cmp::Ordering::Less => left = middle + 1,
                std::cmp::Ordering::Greater => right = middle,
                std::cmp::Ordering::Equal => {
                    let posting_offset = read_u64(&self.lookup, offset + 9)? as usize;
                    let count = read_u32(&self.lookup, offset + 17)? as usize;
                    let start = header_length() + posting_offset + 4;
                    return Some((start, count));
                }
            }
        }
        None
    }
}

fn posting_contains(bytes: &[u8], start: usize, count: usize, target: u32) -> bool {
    let mut left = 0usize;
    let mut right = count;
    while left < right {
        let middle = left + (right - left) / 2;
        let Some(id) = read_u32(bytes, start + middle * 4) else {
            return false;
        };
        match id.cmp(&target) {
            std::cmp::Ordering::Less => left = middle + 1,
            std::cmp::Ordering::Greater => right = middle,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

pub(crate) fn load(storage: &FastRegexSearchStorage) -> Result<Option<IndexState>, FastRegexError> {
    let FastRegexSearchStorage::Persistent(directory) = storage else {
        return Ok(None);
    };
    let complete_path = directory.join(COMPLETE_FILE);
    let Some(generation) = read_complete(&complete_path)? else {
        return Ok(None);
    };
    let (source_bytes, documents, document_paths, ids) =
        read_documents(&directory.join(DOCUMENTS_FILE), generation)?;
    let (pair_weights, folded_pair_weights) =
        read_weights(&directory.join(WEIGHTS_FILE), generation)?;
    let disk_base = DiskBaseIndex::open(
        &directory.join(LOOKUP_FILE),
        &directory.join(POSTINGS_FILE),
        generation,
        ids,
    )?;
    let mut state = IndexState {
        generation,
        base_generation: generation,
        source_bytes,
        documents,
        next_document_id: document_paths.len() as u32,
        document_paths,
        postings: HashMap::new(),
        folded_postings: HashMap::new(),
        overlays: BTreeMap::new(),
        dirty_paths: BTreeSet::new(),
        disk_base: Some(disk_base),
        pair_weights,
        folded_pair_weights,
    };
    apply_delta(&directory.join(DELTA_FILE), &mut state)?;
    Ok(Some(state))
}

pub(crate) fn persist(
    storage: &FastRegexSearchStorage,
    state: &IndexState,
) -> Result<(), FastRegexError> {
    let FastRegexSearchStorage::Persistent(directory) = storage else {
        return Ok(());
    };
    let mut ids = BTreeMap::new();
    let mut documents = generation_header(state.generation);
    documents.extend_from_slice(&(state.source_bytes as u64).to_le_bytes());
    documents.extend_from_slice(&(state.documents.len() as u64).to_le_bytes());
    for (id, (path, document)) in state.documents.iter().enumerate() {
        ids.insert(document.id, id as u32);
        write_bytes(&mut documents, path.to_string_lossy().as_bytes());
        documents.extend_from_slice(&(document.source_bytes as u64).to_le_bytes());
        write_bytes(&mut documents, document.revision.as_bytes());
    }

    let mut weights = generation_header(state.generation);
    for count in state.pair_weights.counts() {
        weights.extend_from_slice(&count.to_le_bytes());
    }
    for count in state.folded_pair_weights.counts() {
        weights.extend_from_slice(&count.to_le_bytes());
    }

    let mut postings = generation_header(state.generation);
    let mut lookup = generation_header(state.generation);
    let sensitive = sorted_postings(&state.postings);
    let folded = sorted_postings(&state.folded_postings);
    lookup.extend_from_slice(&((sensitive.len() + folded.len()) as u64).to_le_bytes());
    write_posting_entries(&mut lookup, &mut postings, &ids, false, sensitive);
    write_posting_entries(&mut lookup, &mut postings, &ids, true, folded);

    write_atomic(directory.join(DOCUMENTS_FILE), &documents)?;
    write_atomic(directory.join(WEIGHTS_FILE), &weights)?;
    write_atomic(directory.join(POSTINGS_FILE), &postings)?;
    write_atomic(directory.join(LOOKUP_FILE), &lookup)?;
    write_atomic(
        directory.join(DELTA_FILE),
        &delta_header(state.generation, state.generation, 0),
    )?;
    write_atomic(
        directory.join(COMPLETE_FILE),
        &generation_header(state.generation),
    )
}

pub(crate) fn persist_delta(
    storage: &FastRegexSearchStorage,
    state: &IndexState,
) -> Result<(), FastRegexError> {
    let FastRegexSearchStorage::Persistent(directory) = storage else {
        return Ok(());
    };
    let mut bytes = delta_header(
        state.base_generation,
        state.generation,
        state.dirty_paths.len(),
    );
    for path in &state.dirty_paths {
        write_bytes(&mut bytes, path.to_string_lossy().as_bytes());
        let Some(document) = state.documents.get(path) else {
            bytes.push(0);
            continue;
        };
        bytes.push(1);
        bytes.extend_from_slice(&(document.source_bytes as u64).to_le_bytes());
        write_bytes(&mut bytes, document.revision.as_bytes());
        write_grams(&mut bytes, &document.grams);
        write_grams(&mut bytes, &document.folded_grams);
    }
    write_atomic(directory.join(DELTA_FILE), &bytes)
}

fn apply_delta(path: &Path, state: &mut IndexState) -> Result<(), FastRegexError> {
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    let mut reader = Reader::new(path, &bytes);
    if !reader.take(STORE_VERSION.len())?.eq(STORE_VERSION) {
        return Err(corrupt(path));
    }
    let base_generation = reader.u64()?;
    let current_generation = reader.u64()?;
    let changed_count = reader.usize()?;
    if base_generation != state.base_generation || current_generation < base_generation {
        return Err(corrupt(path));
    }
    for _ in 0..changed_count {
        let changed_path = PathBuf::from(reader.string()?);
        remove_stored_document(state, &changed_path);
        match reader.u8()? {
            0 => {}
            1 => {
                let document = IndexedDocument {
                    id: 0,
                    source_bytes: reader.usize()?,
                    revision: reader.string()?,
                    grams: reader.grams()?,
                    folded_grams: reader.grams()?,
                };
                insert_stored_document(state, changed_path.clone(), document);
            }
            _ => return Err(corrupt(path)),
        }
        state.dirty_paths.insert(changed_path);
    }
    reader.finish()?;
    state.generation = current_generation;
    Ok(())
}

fn remove_stored_document(state: &mut IndexState, path: &Path) {
    let Some(document) = state.documents.remove(path) else {
        return;
    };
    state.source_bytes = state.source_bytes.saturating_sub(document.source_bytes);
    state.document_paths.remove(&document.id);
    remove_stored_postings(&mut state.postings, document.id, &document.grams);
    remove_stored_postings(
        &mut state.folded_postings,
        document.id,
        &document.folded_grams,
    );
}

fn remove_stored_postings(postings: &mut HashMap<u64, BTreeSet<u32>>, id: u32, grams: &[u64]) {
    for gram in grams {
        if let Some(ids) = postings.get_mut(gram) {
            ids.remove(&id);
            if ids.is_empty() {
                postings.remove(gram);
            }
        }
    }
}

fn insert_stored_document(state: &mut IndexState, path: PathBuf, mut document: IndexedDocument) {
    let id = state.next_document_id;
    state.next_document_id = state.next_document_id.saturating_add(1);
    document.id = id;
    for gram in &document.grams {
        state.postings.entry(*gram).or_default().insert(id);
    }
    for gram in &document.folded_grams {
        state.folded_postings.entry(*gram).or_default().insert(id);
    }
    state.source_bytes = state.source_bytes.saturating_add(document.source_bytes);
    state.document_paths.insert(id, path.clone());
    state.documents.insert(path, document);
}

fn sorted_postings(postings: &HashMap<u64, BTreeSet<u32>>) -> Vec<(&u64, &BTreeSet<u32>)> {
    let mut entries = postings.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(gram, _)| **gram);
    entries
}

fn write_posting_entries(
    lookup: &mut Vec<u8>,
    postings: &mut Vec<u8>,
    ids: &BTreeMap<u32, u32>,
    folded: bool,
    entries: Vec<(&u64, &BTreeSet<u32>)>,
) {
    for (gram, document_ids) in entries {
        let offset = (postings.len() - STORE_VERSION.len() - 8) as u64;
        postings.extend_from_slice(&(document_ids.len() as u32).to_le_bytes());
        for document_id in document_ids {
            postings.extend_from_slice(&ids[document_id].to_le_bytes());
        }
        lookup.push(u8::from(folded));
        lookup.extend_from_slice(&gram.to_le_bytes());
        lookup.extend_from_slice(&offset.to_le_bytes());
        lookup.extend_from_slice(&(document_ids.len() as u32).to_le_bytes());
    }
}

type LoadedDocuments = (
    usize,
    BTreeMap<PathBuf, IndexedDocument>,
    BTreeMap<u32, PathBuf>,
    Vec<PathBuf>,
);

fn read_documents(path: &Path, generation: u64) -> Result<LoadedDocuments, FastRegexError> {
    let bytes = read_generation_file(path, generation)?;
    let mut reader = Reader::new(path, &bytes);
    let source_bytes = reader.usize()?;
    let document_count = reader.usize()?;
    let mut documents = BTreeMap::new();
    let mut document_paths = BTreeMap::new();
    let mut ids = Vec::with_capacity(document_count);
    for id in 0..document_count {
        let path = PathBuf::from(reader.string()?);
        let source_bytes = reader.usize()?;
        let revision = reader.string()?;
        if documents.contains_key(&path) {
            return Err(corrupt(reader.path));
        }
        let id = u32::try_from(id).map_err(|_| corrupt(reader.path))?;
        ids.push(path.clone());
        document_paths.insert(id, path.clone());
        documents.insert(
            path,
            IndexedDocument {
                id,
                revision,
                source_bytes,
                grams: Vec::new(),
                folded_grams: Vec::new(),
            },
        );
    }
    reader.finish()?;
    Ok((source_bytes, documents, document_paths, ids))
}

fn read_weights(
    path: &Path,
    generation: u64,
) -> Result<(PairWeights, PairWeights), FastRegexError> {
    let bytes = read_generation_file(path, generation)?;
    let mut reader = Reader::new(path, &bytes);
    let mut sensitive = Box::new([0u32; 1 << 16]);
    let mut folded = Box::new([0u32; 1 << 16]);
    for count in sensitive.iter_mut() {
        *count = reader.u32()?;
    }
    for count in folded.iter_mut() {
        *count = reader.u32()?;
    }
    reader.finish()?;
    Ok((
        PairWeights::from_counts(sensitive),
        PairWeights::from_counts(folded),
    ))
}

fn validate_disk_postings(
    path: &Path,
    lookup: &[u8],
    postings: &[u8],
    path_count: usize,
) -> Result<(), FastRegexError> {
    let entry_count = read_u64(lookup, header_length()).ok_or_else(|| corrupt(path))? as usize;
    let entries_offset = header_length() + 8;
    let lookup_end = entries_offset
        .checked_add(entry_count.checked_mul(21).ok_or_else(|| corrupt(path))?)
        .ok_or_else(|| corrupt(path))?;
    if lookup_end != lookup.len() {
        return Err(corrupt(path));
    }
    let mut previous = None;
    for index in 0..entry_count {
        let offset = entries_offset + index * 21;
        let folded = lookup[offset];
        if folded > 1 {
            return Err(corrupt(path));
        }
        let gram = read_u64(lookup, offset + 1).ok_or_else(|| corrupt(path))?;
        let key = (folded, gram);
        if previous.is_some_and(|previous| previous >= key) {
            return Err(corrupt(path));
        }
        previous = Some(key);
        let posting_offset = read_u64(lookup, offset + 9).ok_or_else(|| corrupt(path))? as usize;
        let expected_count = read_u32(lookup, offset + 17).ok_or_else(|| corrupt(path))? as usize;
        let posting_start = header_length()
            .checked_add(posting_offset)
            .ok_or_else(|| corrupt(path))?;
        let actual_count = read_u32(postings, posting_start).ok_or_else(|| corrupt(path))? as usize;
        if actual_count != expected_count {
            return Err(corrupt(path));
        }
        let ids_start = posting_start + 4;
        let ids_end = ids_start
            .checked_add(actual_count.checked_mul(4).ok_or_else(|| corrupt(path))?)
            .ok_or_else(|| corrupt(path))?;
        if ids_end > postings.len() {
            return Err(corrupt(path));
        }
        let mut previous_id = None;
        for id_index in 0..actual_count {
            let id =
                read_u32(postings, ids_start + id_index * 4).ok_or_else(|| corrupt(path))? as usize;
            if id >= path_count || previous_id.is_some_and(|previous| previous >= id) {
                return Err(corrupt(path));
            }
            previous_id = Some(id);
        }
    }
    Ok(())
}

fn read_complete_generation_file(path: &Path, generation: u64) -> Result<Vec<u8>, FastRegexError> {
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    if bytes.len() < header_length()
        || &bytes[..STORE_VERSION.len()] != STORE_VERSION
        || read_u64(&bytes, STORE_VERSION.len()) != Some(generation)
    {
        return Err(corrupt(path));
    }
    Ok(bytes)
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let value = bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(value))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let value = bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(value))
}

fn header_length() -> usize {
    STORE_VERSION.len() + 8
}

fn read_complete(path: &Path) -> Result<Option<u64>, FastRegexError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io_error(path, source)),
    };
    let mut reader = Reader::new(path, &bytes);
    if !reader.take(STORE_VERSION.len())?.eq(STORE_VERSION) {
        return Ok(None);
    }
    let generation = reader.u64()?;
    reader.finish()?;
    Ok(Some(generation))
}

fn read_generation_file(path: &Path, generation: u64) -> Result<Vec<u8>, FastRegexError> {
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    let mut reader = Reader::new(path, &bytes);
    if !reader.take(STORE_VERSION.len())?.eq(STORE_VERSION) || reader.u64()? != generation {
        return Err(corrupt(path));
    }
    Ok(reader.remaining().to_vec())
}

fn generation_header(generation: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(STORE_VERSION.len() + 8);
    bytes.extend_from_slice(STORE_VERSION);
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes
}

fn delta_header(base_generation: u64, generation: u64, changed_count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(STORE_VERSION.len() + 24);
    bytes.extend_from_slice(STORE_VERSION);
    bytes.extend_from_slice(&base_generation.to_le_bytes());
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes.extend_from_slice(&(changed_count as u64).to_le_bytes());
    bytes
}

fn write_bytes(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u32).to_le_bytes());
    output.extend_from_slice(value);
}

fn write_grams(output: &mut Vec<u8>, grams: &[u64]) {
    output.extend_from_slice(&(grams.len() as u64).to_le_bytes());
    for gram in grams {
        output.extend_from_slice(&gram.to_le_bytes());
    }
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

struct Reader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(path: &'a Path, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            offset: 0,
        }
    }

    fn take(&mut self, length: usize) -> Result<&[u8], FastRegexError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| corrupt(self.path))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, FastRegexError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, FastRegexError> {
        let bytes = self.take(4)?.try_into().expect("four bytes");
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, FastRegexError> {
        let bytes = self.take(8)?.try_into().expect("eight bytes");
        Ok(u64::from_le_bytes(bytes))
    }

    fn usize_from_u32(&mut self) -> Result<usize, FastRegexError> {
        usize::try_from(self.u32()?).map_err(|_| corrupt(self.path))
    }

    fn usize(&mut self) -> Result<usize, FastRegexError> {
        usize::try_from(self.u64()?).map_err(|_| corrupt(self.path))
    }

    fn string(&mut self) -> Result<String, FastRegexError> {
        let length = self.usize_from_u32()?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| corrupt(self.path))
    }

    fn grams(&mut self) -> Result<Vec<u64>, FastRegexError> {
        let count = self.usize()?;
        let byte_count = count.checked_mul(8).ok_or_else(|| corrupt(self.path))?;
        if byte_count > self.remaining().len() {
            return Err(corrupt(self.path));
        }
        let mut grams = Vec::with_capacity(count);
        for _ in 0..count {
            grams.push(self.u64()?);
        }
        Ok(grams)
    }

    fn remaining(&self) -> &[u8] {
        &self.bytes[self.offset..]
    }

    fn finish(&self) -> Result<(), FastRegexError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(corrupt(self.path))
        }
    }
}

fn corrupt(path: &Path) -> FastRegexError {
    FastRegexError::CorruptIndex(path.to_path_buf())
}

fn io_error(path: &Path, source: std::io::Error) -> FastRegexError {
    FastRegexError::Io {
        path: path.to_path_buf(),
        source,
    }
}
