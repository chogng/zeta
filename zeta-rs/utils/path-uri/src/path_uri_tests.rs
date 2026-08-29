use super::*;
use pretty_assertions::assert_eq;

#[test]
fn parse_accepts_only_plain_file_uris() {
    assert!(PathUri::parse("file:///workspace/src").is_ok());
    assert_eq!(
        PathUri::parse("https://example.com/file").unwrap_err(),
        PathUriParseError::UnsupportedScheme("https".into())
    );
    assert_eq!(
        PathUri::parse("file:///workspace?revision=1").unwrap_err(),
        PathUriParseError::QueryNotAllowed
    );
    assert_eq!(
        PathUri::parse("file:///workspace#anchor").unwrap_err(),
        PathUriParseError::FragmentNotAllowed
    );
}

#[test]
fn localhost_and_windows_drive_letters_are_canonicalized() {
    assert_eq!(
        PathUri::parse("file://localhost/tmp/project")
            .unwrap()
            .to_string(),
        "file:///tmp/project"
    );
    assert_eq!(
        PathUri::parse("file:///c:/Users/Zeta").unwrap().to_string(),
        "file:///C:/Users/Zeta"
    );
}

#[test]
fn serde_uses_the_canonical_uri_string() {
    let uri = PathUri::parse("file:///c:/An%20item").unwrap();
    let json = serde_json::to_string(&uri).unwrap();

    assert_eq!(json, r#""file:///C:/An%20item""#);
    assert_eq!(serde_json::from_str::<PathUri>(&json).unwrap(), uri);
}

#[test]
fn explicit_native_conventions_are_host_independent() {
    let posix = PathUri::from_native_path("/workspace/src/main.rs", PathConvention::Posix).unwrap();
    let windows =
        PathUri::from_native_path(r"c:\workspace\src\main.rs", PathConvention::Windows).unwrap();
    let unc =
        PathUri::from_native_path(r"\\server\share\src\main.rs", PathConvention::Windows).unwrap();

    assert_eq!(posix.to_string(), "file:///workspace/src/main.rs");
    assert_eq!(windows.to_string(), "file:///C:/workspace/src/main.rs");
    assert_eq!(unc.to_string(), "file://server/share/src/main.rs");
    assert_eq!(
        posix.inferred_native_path_string(),
        "/workspace/src/main.rs"
    );
    assert_eq!(
        windows.inferred_native_path_string(),
        r"C:\workspace\src\main.rs"
    );
    assert_eq!(
        unc.inferred_native_path_string(),
        r"\\server\share\src\main.rs"
    );
}

#[test]
fn relative_and_malformed_native_paths_are_rejected() {
    assert!(matches!(
        PathUri::from_native_path("src/main.rs", PathConvention::Posix),
        Err(PathUriParseError::InvalidNativePath { .. })
    ));
    assert!(matches!(
        PathUri::from_native_path(r"C:src\main.rs", PathConvention::Windows),
        Err(PathUriParseError::InvalidNativePath { .. })
    ));
    assert!(matches!(
        PathUri::from_native_path(r"\\server", PathConvention::Windows),
        Err(PathUriParseError::InvalidNativePath { .. })
    ));
}

#[test]
fn basename_parent_and_ancestors_stop_at_the_native_root() {
    let uri = PathUri::parse("file:///C:/workspace/src/main.rs").unwrap();

    assert_eq!(uri.basename().as_deref(), Some("main.rs"));
    assert_eq!(
        uri.parent().unwrap().to_string(),
        "file:///C:/workspace/src"
    );
    assert_eq!(
        uri.ancestors()
            .map(|uri| uri.to_string())
            .collect::<Vec<_>>(),
        vec![
            "file:///C:/workspace/src/main.rs",
            "file:///C:/workspace/src",
            "file:///C:/workspace",
            "file:///C:/",
        ]
    );
}

#[test]
fn containment_uses_authority_and_segment_boundaries() {
    let root = PathUri::parse("file:///workspace").unwrap();
    let child = PathUri::parse("file:///workspace/src/main.rs").unwrap();
    let sibling_prefix = PathUri::parse("file:///workspace-other/file").unwrap();
    let remote = PathUri::parse("file://server/workspace/src").unwrap();

    assert!(child.starts_with(&root));
    assert!(!sibling_prefix.starts_with(&root));
    assert!(!remote.starts_with(&root));
    assert_eq!(
        child.relative_path_from(&root).as_deref(),
        Some("src/main.rs")
    );
}

#[test]
fn containment_rejects_encoded_native_separators() {
    let root = PathUri::parse("file:///workspace").unwrap();
    let encoded_posix = PathUri::parse("file:///workspace/src%2Fsecret").unwrap();
    let windows_root = PathUri::parse("file:///C:/workspace").unwrap();
    let encoded_windows = PathUri::parse("file:///C:/workspace/src%5Csecret").unwrap();

    assert!(!encoded_posix.starts_with(&root));
    assert!(!encoded_windows.starts_with(&windows_root));
}

#[test]
fn join_normalizes_components_without_escaping_roots() {
    let posix = PathUri::parse("file:///workspace/src").unwrap();
    let windows = PathUri::parse("file:///C:/workspace/src").unwrap();

    assert_eq!(
        posix.join("../tests/./case.rs").unwrap().to_string(),
        "file:///workspace/tests/case.rs"
    );
    assert_eq!(
        posix.join("../../../../etc").unwrap().to_string(),
        "file:///etc"
    );
    assert_eq!(
        windows.join(r"\other\file.rs").unwrap().to_string(),
        "file:///C:/other/file.rs"
    );
    assert!(windows.join(r"D:relative").is_err());
}

#[test]
fn host_paths_round_trip_on_the_current_platform() {
    let directory = tempfile::tempdir().unwrap();
    let path = AbsolutePathBuf::from_absolute(directory.path()).unwrap();
    let uri = PathUri::from_absolute_path(&path);

    assert_eq!(uri.to_host_path().unwrap(), path);
}

#[cfg(unix)]
#[test]
fn non_utf8_host_paths_round_trip_losslessly() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let path = std::path::PathBuf::from(OsString::from_vec(b"/tmp/zeta-\xFF".to_vec()));
    let path = AbsolutePathBuf::from_absolute(path).unwrap();
    let uri = PathUri::from_absolute_path(&path);

    assert!(uri.to_string().contains("%FF"));
    assert_eq!(uri.to_host_path().unwrap(), path);
    assert_eq!(uri.basename().as_deref(), Some("zeta-%FF"));
}
