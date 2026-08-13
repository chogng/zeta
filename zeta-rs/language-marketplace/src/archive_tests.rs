use std::io::Cursor;
use std::io::Write;

use tempfile::TempDir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::LanguageMarketplaceErrorKind;
use crate::archive;

#[test]
fn bounded_archive_extracts_exact_signed_file_statistics() {
    let bytes = archive_bytes(&[("README.md", b"hello"), ("server/main", b"world")]);
    let root = TempDir::new().unwrap();

    archive::extract(&bytes, root.path(), 2, 10).unwrap();
    assert_eq!(
        std::fs::read(root.path().join("server/main")).unwrap(),
        b"world"
    );
}

#[test]
fn archive_rejects_path_escape_and_signed_stat_mismatch() {
    let escaping = archive_bytes(&[("../escape", b"no")]);
    let root = TempDir::new().unwrap();
    assert_eq!(
        archive::extract(&escaping, root.path(), 1, 2)
            .unwrap_err()
            .kind(),
        LanguageMarketplaceErrorKind::PackageUnsafe
    );

    let ordinary = archive_bytes(&[("README.md", b"hello")]);
    assert_eq!(
        archive::extract(&ordinary, root.path(), 2, 5)
            .unwrap_err()
            .kind(),
        LanguageMarketplaceErrorKind::PackageUnsafe
    );
}

fn archive_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in files {
        archive
            .start_file(*path, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}
