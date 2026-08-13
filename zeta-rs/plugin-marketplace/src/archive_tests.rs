use std::io::Cursor;
use std::io::Write;

use tempfile::tempdir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::RemoteMarketplaceErrorKind;
use crate::archive;

#[test]
fn archive_extracts_regular_files() {
    let bytes = archive_with([(".zeta-plugin/plugin.json", b"{}".as_slice())]);
    let root = tempdir().unwrap();

    archive::extract(&bytes, root.path()).unwrap();

    assert_eq!(
        std::fs::read(root.path().join(".zeta-plugin/plugin.json")).unwrap(),
        b"{}"
    );
}

#[test]
fn archive_rejects_parent_traversal() {
    let bytes = archive_with([("../outside", b"bad".as_slice())]);
    let root = tempdir().unwrap();

    let error = archive::extract(&bytes, root.path()).unwrap_err();

    assert_eq!(error.kind(), RemoteMarketplaceErrorKind::PackageUnsafe);
    assert!(!root.path().parent().unwrap().join("outside").exists());
}

#[test]
fn archive_rejects_backslash_paths() {
    let bytes = archive_with([("..\\outside", b"bad".as_slice())]);
    let root = tempdir().unwrap();

    let error = archive::extract(&bytes, root.path()).unwrap_err();

    assert_eq!(error.kind(), RemoteMarketplaceErrorKind::PackageUnsafe);
}

#[test]
fn archive_rejects_duplicate_entries() {
    let mut output = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default();
        for (name, contents) in [
            ("same", b"first".as_slice()),
            ("./same", b"second".as_slice()),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(contents).unwrap();
        }
        zip.finish().unwrap();
    }
    let root = tempdir().unwrap();

    let error = archive::extract(output.get_ref(), root.path()).unwrap_err();

    assert_eq!(error.kind(), RemoteMarketplaceErrorKind::PackageUnsafe);
}

fn archive_with<const N: usize>(files: [(&str, &[u8]); N]) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default();
        for (name, contents) in files {
            zip.start_file(name, options).unwrap();
            zip.write_all(contents).unwrap();
        }
        zip.finish().unwrap();
    }
    output.into_inner()
}
