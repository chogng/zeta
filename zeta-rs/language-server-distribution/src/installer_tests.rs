use super::*;

fn package(version: &str, bytes: &[u8]) -> LanguageServerPackage {
    LanguageServerPackage::new(
        "rust-analyzer",
        version,
        "bin/rust-analyzer",
        vec![
            LanguageServerPackageFile::regular("README.txt", b"managed package".to_vec()).unwrap(),
            LanguageServerPackageFile::executable("bin/rust-analyzer", bytes.to_vec()).unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn verified_versions_install_side_by_side_and_repeat_idempotently() {
    let root = tempfile::tempdir().unwrap();
    let installer = LanguageServerInstaller::new(root.path()).unwrap();
    let first = package("1.0.0", b"first");
    let first_digest = first.sha256();
    let installed = installer
        .install_verified(first.clone(), first_digest)
        .unwrap();
    assert_eq!(fs::read(&installed.executable).unwrap(), b"first");
    assert_eq!(
        installer.install_verified(first, first_digest).unwrap(),
        installed
    );

    let second = package("2.0.0", b"second");
    let second_digest = second.sha256();
    let updated = installer.install_verified(second, second_digest).unwrap();
    assert_eq!(fs::read(&updated.executable).unwrap(), b"second");
    assert!(installed.executable.exists());
}

#[test]
fn digest_mismatch_and_package_traversal_fail_without_publishing() {
    assert!(matches!(
        LanguageServerPackageFile::regular("../escape", Vec::new()),
        Err(LanguageServerDistributionError::InvalidPackagePath(_))
    ));
    let root = tempfile::tempdir().unwrap();
    let installer = LanguageServerInstaller::new(root.path()).unwrap();
    let package = package("1.0.0", b"server");
    assert!(matches!(
        installer.install_verified(package, [0; 32]),
        Err(LanguageServerDistributionError::DigestMismatch)
    ));
    assert!(!root.path().join("rust-analyzer/1.0.0").exists());
    assert!(matches!(
        LanguageServerPackage::new(
            "..",
            "1.0.0",
            "server",
            vec![LanguageServerPackageFile::executable("server", b"x".to_vec()).unwrap()],
        ),
        Err(LanguageServerDistributionError::InvalidIdentity { .. })
    ));
}

#[test]
fn existing_installation_is_reverified_instead_of_trusting_its_receipt() {
    let root = tempfile::tempdir().unwrap();
    let installer = LanguageServerInstaller::new(root.path()).unwrap();
    let package = package("1.0.0", b"trusted");
    let digest = package.sha256();
    let installed = installer.install_verified(package.clone(), digest).unwrap();
    fs::write(&installed.executable, b"tampered").unwrap();

    assert!(matches!(
        installer.install_verified(package, digest),
        Err(LanguageServerDistributionError::ExistingInstallationMismatch)
    ));
}

#[test]
fn package_digest_binds_the_declared_executable_path() {
    let files = vec![
        LanguageServerPackageFile::executable("bin/server", b"same".to_vec()).unwrap(),
        LanguageServerPackageFile::executable("bin/helper", b"same".to_vec()).unwrap(),
    ];
    let server = LanguageServerPackage::new("server", "1", "bin/server", files.clone()).unwrap();
    let helper = LanguageServerPackage::new("server", "1", "bin/helper", files).unwrap();

    assert_ne!(server.sha256(), helper.sha256());
}
