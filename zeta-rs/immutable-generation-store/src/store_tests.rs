use super::GenerationFile;
use super::ImmutableGenerationStore;
use std::fs;

#[test]
fn publishes_and_reads_a_consistent_base_and_layer_snapshot() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            1,
            &[GenerationFile::new("lookup.bin", b"lookup one")],
            &[GenerationFile::new("delta.bin", b"delta one")],
        )
        .expect("publish base");

    let snapshot = store
        .open_current()
        .expect("read snapshot")
        .expect("published snapshot");
    assert_eq!(snapshot.generation(), 1);
    assert_eq!(snapshot.base_generation(), 1);
    let lookup = snapshot.map_base("lookup.bin").expect("mapped lookup");
    let delta = snapshot.read_layer("delta.bin").expect("delta");
    let value = (lookup.as_slice().to_vec(), delta);

    assert_eq!(value, (b"lookup one".to_vec(), b"delta one".to_vec()));
}

#[test]
fn layer_publication_reuses_the_immutable_base() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            4,
            &[GenerationFile::new("lookup.bin", b"base four")],
            &[GenerationFile::new("delta.bin", b"delta four")],
        )
        .expect("publish base");
    store
        .publish_layer(5, &[GenerationFile::new("delta.bin", b"delta five")])
        .expect("publish layer");

    let snapshot = store.open_current().unwrap().unwrap();
    let value = (
        snapshot.generation(),
        snapshot.base_generation(),
        snapshot.read_base("lookup.bin").unwrap(),
        snapshot.read_layer("delta.bin").unwrap(),
    );

    assert_eq!(value, (5, 4, b"base four".to_vec(), b"delta five".to_vec()));
}

#[test]
fn mapped_reader_keeps_an_old_base_until_its_lease_is_dropped() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            1,
            &[GenerationFile::new("lookup.bin", b"old lookup")],
            &[GenerationFile::new("delta.bin", b"old delta")],
        )
        .expect("old base");
    let old_mapping = store
        .open_current()
        .unwrap()
        .unwrap()
        .map_base("lookup.bin")
        .unwrap();

    store
        .publish_base(
            2,
            &[GenerationFile::new("lookup.bin", b"new lookup")],
            &[GenerationFile::new("delta.bin", b"new delta")],
        )
        .expect("new base");

    assert_eq!(old_mapping.as_slice(), b"old lookup");
    assert!(directory.path().join("bases/00000000000000000001").exists());
    drop(old_mapping);
    store
        .publish_layer(3, &[GenerationFile::new("delta.bin", b"third delta")])
        .expect("trigger cleanup");
    assert!(!directory.path().join("bases/00000000000000000001").exists());
}

#[test]
fn incomplete_directories_do_not_become_current() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    let pending = directory.path().join("bases/.pending-00000000000000000001");
    fs::create_dir(&pending).expect("pending directory");
    fs::write(pending.join("lookup.bin"), b"incomplete").expect("pending file");

    assert!(store.open_current().unwrap().is_none());
}

#[test]
fn a_newer_directory_without_a_manifest_does_not_replace_the_current_snapshot() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            1,
            &[GenerationFile::new("lookup.bin", b"published")],
            &[GenerationFile::new("delta.bin", b"published delta")],
        )
        .expect("published base");
    let orphan = directory.path().join("layers/00000000000000000002");
    fs::create_dir(&orphan).expect("orphan layer");
    fs::write(orphan.join(".lease"), []).expect("orphan lease");
    fs::write(orphan.join("delta.bin"), b"not published").expect("orphan delta");

    let snapshot = store.open_current().unwrap().unwrap();

    assert_eq!(snapshot.generation(), 1);
    assert_eq!(
        snapshot.read_layer("delta.bin").unwrap(),
        b"published delta"
    );
}

#[test]
fn an_open_snapshot_keeps_its_layer_until_the_snapshot_is_dropped() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            1,
            &[GenerationFile::new("lookup.bin", b"lookup")],
            &[GenerationFile::new("delta.bin", b"first")],
        )
        .expect("base");
    let old_snapshot = store.open_current().unwrap().unwrap();

    store
        .publish_layer(2, &[GenerationFile::new("delta.bin", b"second")])
        .expect("second layer");

    assert_eq!(old_snapshot.read_layer("delta.bin").unwrap(), b"first");
    assert!(
        directory
            .path()
            .join("layers/00000000000000000001")
            .exists()
    );
    drop(old_snapshot);
    store
        .publish_layer(3, &[GenerationFile::new("delta.bin", b"third")])
        .expect("third layer");
    assert!(
        !directory
            .path()
            .join("layers/00000000000000000001")
            .exists()
    );
}

#[test]
fn empty_files_can_be_read_but_are_rejected_for_mapping() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            1,
            &[GenerationFile::new("empty.bin", b"")],
            &[GenerationFile::new("delta.bin", b"delta")],
        )
        .expect("base");
    let snapshot = store.open_current().unwrap().unwrap();

    assert!(snapshot.read_base("empty.bin").unwrap().is_empty());
    assert_eq!(
        snapshot.map_base("empty.bin").unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[test]
fn retries_a_generation_left_unpublished_after_interruption() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    let orphan = directory.path().join("bases/00000000000000000001");
    fs::create_dir(&orphan).expect("orphan base");
    fs::write(orphan.join(".lease"), []).expect("orphan lease");
    fs::write(orphan.join("lookup.bin"), b"partial").expect("orphan lookup");

    store
        .publish_base(
            1,
            &[GenerationFile::new("lookup.bin", b"complete")],
            &[GenerationFile::new("delta.bin", b"delta")],
        )
        .expect("retry publication");

    let snapshot = store.open_current().unwrap().unwrap();
    assert_eq!(snapshot.read_base("lookup.bin").unwrap(), b"complete");
}

#[test]
fn rejects_non_monotonic_snapshots_and_unsafe_file_names() {
    let directory = tempfile::tempdir().expect("store directory");
    let store = ImmutableGenerationStore::open(directory.path()).expect("store");
    store
        .publish_base(
            8,
            &[GenerationFile::new("lookup.bin", b"lookup")],
            &[GenerationFile::new("delta.bin", b"delta")],
        )
        .expect("base");

    let repeated = store.publish_layer(8, &[GenerationFile::new("delta.bin", b"repeated")]);
    let escaped = store.publish_layer(9, &[GenerationFile::new("../delta.bin", b"escaped")]);

    assert_eq!(
        repeated.unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        escaped.unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}
