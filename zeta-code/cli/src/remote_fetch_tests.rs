use std::num::NonZeroU64;
use std::path::PathBuf;

use zeta_remote::RemotePlatform;
use zeta_remote_connections::RemoteRuntimeArtifact;
use zeta_remote_connections::RemoteRuntimeArtifactIntegrity;
use zeta_remote_connections::RemoteRuntimeDownloadDisposition;
use zeta_remote_connections::RemoteRuntimeDownloadProgress;
use zeta_remote_connections::RemoteRuntimeVersion;

use super::RemoteRuntimeArtifactOutput;
use super::progress_json;

#[test]
fn artifact_output_preserves_all_install_integrity_fields() {
    let artifact = RemoteRuntimeArtifact::new(
        PathBuf::from("/cache/runtime.tar.gz"),
        RemoteRuntimeVersion::parse("1.2.3").unwrap(),
        RemotePlatform::from_target_triple("x86_64-unknown-linux-gnu").unwrap(),
        RemoteRuntimeArtifactIntegrity::new(
            NonZeroU64::new(42).unwrap(),
            NonZeroU64::new(84).unwrap(),
            "a".repeat(64),
        )
        .unwrap(),
    );

    assert_eq!(
        serde_json::to_string(&RemoteRuntimeArtifactOutput::from(&artifact)).unwrap(),
        format!(
            r#"{{"archivePath":"/cache/runtime.tar.gz","version":"1.2.3","target":"x86_64-unknown-linux-gnu","archiveSize":42,"unpackedSize":84,"sha256":"{}"}}"#,
            "a".repeat(64)
        )
    );
}

#[test]
fn download_progress_is_a_stable_json_lines_record() {
    assert_eq!(
        progress_json(RemoteRuntimeDownloadProgress::DownloadingArtifact {
            transferred_bytes: 32,
            total_bytes: 64,
        }),
        r#"{"kind":"remoteRuntimeDownloadProgress","phase":"downloadingArtifact","transferredBytes":32,"totalBytes":64}"#
    );
    assert_eq!(
        progress_json(RemoteRuntimeDownloadProgress::Complete {
            disposition: RemoteRuntimeDownloadDisposition::Reused,
        }),
        r#"{"kind":"remoteRuntimeDownloadProgress","phase":"complete","disposition":"reused"}"#
    );
}
