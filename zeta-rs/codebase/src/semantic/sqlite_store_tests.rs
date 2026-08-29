use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::ChunkContentHash;
use crate::ChunkKey;
use crate::ChunkReference;
use crate::ChunkSpan;
use crate::Codebase;
use crate::CodebaseLimits;
use crate::CodebaseStorage;
use crate::IndexedLanguage;
use crate::SourceRevision;
use zeta_model_provider::EmbeddingVector;
use zeta_workspace::WorkspaceRoot;

use super::ANN_MIN_CHUNKS;
use super::ANN_REVISION;
use super::SqliteCodebaseVectorStore;
use super::metadata;
use super::set_metadata;
use crate::CodebaseSemanticStorage;
use crate::CodebaseVectorStore;
use crate::EmbeddedCodeChunk;
use crate::EmbeddingIndexKey;

#[test]
fn large_projection_uses_ann_candidates_and_falls_back_when_projection_is_unavailable() {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
    std::fs::write(workspace.path().join("seed.rs"), "fn seed() {}\n").expect("source");
    let index = Codebase::open(
        WorkspaceRoot::open(workspace.path()).expect("root"),
        CodebaseStorage::Memory,
        CodebaseLimits::default(),
    )
    .expect("index");
    let root_id = index.root_id().clone();
    let model = EmbeddingIndexKey::new("ann-test-v1").expect("model");
    let target = 777usize;
    let chunks = (0..ANN_MIN_CHUNKS)
        .map(|index| embedded_chunk(&root_id, index))
        .collect::<Vec<_>>();
    let query = chunks[target].embedding.clone();
    let store = SqliteCodebaseVectorStore::open(&CodebaseSemanticStorage::Memory).expect("store");
    store
        .replace_generation(&root_id, 1, &model, chunks)
        .expect("publish");

    {
        let connection = store.connection.lock().expect("connection");
        assert_eq!(
            metadata(&connection, "ann_revision")
                .expect("metadata")
                .as_deref(),
            Some(ANN_REVISION)
        );
        assert!(
            super::ann_candidate_rowids(&connection, &query, NonZeroUsize::new(5).unwrap())
                .expect("ANN candidates")
                .is_some()
        );
    }
    let approximate = store
        .search(&root_id, 1, &model, &query, NonZeroUsize::new(5).unwrap())
        .expect("ANN search");
    assert_eq!(approximate[0].chunk.reference.relative_path, path(target));

    {
        let connection = store.connection.lock().expect("connection");
        set_metadata(&connection, "ann_revision", "unavailable").expect("invalidate ANN");
    }
    let fallback = store
        .search(&root_id, 1, &model, &query, NonZeroUsize::new(5).unwrap())
        .expect("brute-force fallback");
    assert_eq!(fallback[0].chunk.reference.relative_path, path(target));
}

fn embedded_chunk(root_id: &crate::IndexRootId, index: usize) -> EmbeddedCodeChunk {
    EmbeddedCodeChunk {
        reference: ChunkReference {
            root_id: root_id.clone(),
            relative_path: path(index),
            source_revision: digest(index),
            key: ChunkKey::parse(digest_text(index.saturating_add(10_000))).expect("key"),
            content_hash: ChunkContentHash::parse(digest_text(index.saturating_add(20_000)))
                .expect("hash"),
            span: ChunkSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 0,
                end_line_exclusive: 1,
            },
        },
        language: IndexedLanguage::Rust,
        content: format!("fn item_{index}() {{}}"),
        embedding: vector(index),
    }
}

fn vector(index: usize) -> EmbeddingVector {
    let values = (0..32usize)
        .map(|dimension| {
            let seed = super::mix64(
                (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    ^ (dimension as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9),
            );
            ((seed >> 40) as i32 - (1 << 23)) as f32 / (1 << 23) as f32
        })
        .collect();
    EmbeddingVector::new(values).expect("vector")
}

fn path(index: usize) -> PathBuf {
    PathBuf::from(format!("item-{index}.rs"))
}

fn digest(index: usize) -> SourceRevision {
    SourceRevision::parse(digest_text(index)).expect("revision")
}

fn digest_text(index: usize) -> String {
    format!("sha256:{index:064x}")
}
