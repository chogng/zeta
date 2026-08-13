use std::fs;

use tempfile::TempDir;

use crate::LanguageServerActivationAuthority;
use crate::LanguageServerPackage;
use crate::LanguageServerPackageFile;

#[test]
fn activation_reopens_exact_installation_across_authority_instances() {
    let root = TempDir::new().unwrap();
    let authority = LanguageServerActivationAuthority::open(root.path()).unwrap();
    let package = package("1.0.0", b"server");
    let digest = package.sha256();
    let installed = authority
        .installer()
        .install_verified(package, digest)
        .unwrap();
    let snapshot = authority.activate(installed).unwrap();
    assert_eq!(snapshot.generation(), 2);
    assert_eq!(snapshot.servers()[0].version(), "1.0.0");

    let reopened = LanguageServerActivationAuthority::open(root.path())
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(reopened, snapshot);
}

#[test]
fn activation_refuses_a_mutated_installed_tree() {
    let root = TempDir::new().unwrap();
    let authority = LanguageServerActivationAuthority::open(root.path()).unwrap();
    let package = package("1.0.0", b"server");
    let digest = package.sha256();
    let installed = authority
        .installer()
        .install_verified(package, digest)
        .unwrap();
    authority.activate(installed).unwrap();
    fs::write(
        root.path()
            .join("css-language-server/1.0.0/server/css-language-server"),
        b"tampered",
    )
    .unwrap();

    assert!(LanguageServerActivationAuthority::open(root.path()).is_err());
}

fn package(version: &str, bytes: &[u8]) -> LanguageServerPackage {
    LanguageServerPackage::new(
        "css-language-server",
        version,
        "server/css-language-server",
        vec![
            LanguageServerPackageFile::executable("server/css-language-server", bytes.to_vec())
                .unwrap(),
        ],
    )
    .unwrap()
}
