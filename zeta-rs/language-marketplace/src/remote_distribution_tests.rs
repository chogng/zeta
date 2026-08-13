use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::Ed25519KeyPair;
use jiff::Timestamp;
use semver::Version;
use sha2::Digest;
use sha2::Sha256;
use tempfile::TempDir;
use tough::TargetName;
use tough::editor::RepositoryEditor;
use tough::editor::signed::SignedRole;
use tough::key_source::KeySource;
use tough::key_source::LocalKeySource;
use tough::schema::KeyHolder;
use tough::schema::PathPattern;
use tough::schema::PathSet;
use tough::schema::RoleKeys;
use tough::schema::RoleType;
use tough::schema::Root;
use tough::schema::Target;
use url::Url;
use zeta_http_client::HttpClient;
use zeta_http_client::HttpClientError;
use zeta_http_client::HttpRequest;
use zeta_http_client::HttpResponse;
use zeta_language_server_distribution::LanguageServerActivationAuthority;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::LanguageMarketplaceId;
use crate::RemoteLanguageMarketplace;
use crate::RemoteLanguageMarketplaceConfig;

#[test]
fn signed_tuf_catalog_defers_download_then_installs_and_activates_exact_css() {
    let fixture = SignedDistributionFixture::create();
    let client = Arc::new(FixtureHttpClient {
        root: fixture.repository.path().to_path_buf(),
    });
    let marketplace = RemoteLanguageMarketplace::new(
        RemoteLanguageMarketplaceConfig::new(
            LanguageMarketplaceId::new("test").unwrap(),
            Url::parse("https://marketplace.example/metadata/").unwrap(),
            Url::parse("https://marketplace.example/targets/").unwrap(),
            fixture.trusted_root,
            fixture.cache.path(),
            "zeta",
            Version::new(0, 1, 0),
        )
        .unwrap(),
        client,
    );

    let snapshot = marketplace.sync().unwrap();
    assert_eq!(snapshot.entries().len(), 1);
    assert_eq!(snapshot.entries()[0].server_id(), "css-language-server");
    assert!(!fixture.cache.path().join("packages").exists());

    let authority =
        LanguageServerActivationAuthority::open(fixture.cache.path().join("languages")).unwrap();
    let activation = marketplace
        .install(&snapshot.entries()[0], &authority)
        .unwrap();
    assert_eq!(activation.generation(), 2);
    assert_eq!(activation.servers()[0].server_id(), "css-language-server");
    assert!(activation.servers()[0].executable().is_file());

    fs::write(activation.servers()[0].executable(), b"tampered").unwrap();
    assert!(authority.snapshot().is_err());
}

struct SignedDistributionFixture {
    repository: TempDir,
    cache: TempDir,
    trusted_root: Vec<u8>,
}

impl SignedDistributionFixture {
    fn create() -> Self {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(Self::create_async())
    }

    async fn create_async() -> Self {
        let repository = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let key_path = repository.path().join("key.pk8");
        let key = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        fs::write(&key_path, key.as_ref()).unwrap();
        let keys: Vec<Box<dyn KeySource>> = vec![Box::new(LocalKeySource { path: key_path })];
        let trusted_root_path = repository.path().join("trusted-root.json");
        write_trusted_root(&trusted_root_path, &keys).await;

        let package_root = repository.path().join("package");
        write_css_package(&package_root);
        let (digest, file_count, size_bytes) = package_digest(&package_root);
        let archive_path = repository.path().join("css.zip");
        write_archive(&package_root, &archive_path);
        let language_index_path = repository.path().join("language-index.json");
        fs::write(
            &language_index_path,
            br#"{"schemaVersion":2,"languages":[{"id":"css","displayName":"CSS","fileExtensions":[".css"],"packages":[{"id":"marketplace/css","version":"1.0.0","lsp":true}]}]}"#,
        )
        .unwrap();
        let revocations_path = repository.path().join("revocations.json");
        fs::write(&revocations_path, br#"{"schemaVersion":1,"revoked":[]}"#).unwrap();

        let mut package_target = Target::from_path(&archive_path).await.unwrap();
        package_target.custom.insert(
            "marketplacePackage".into(),
            serde_json::json!({
                "schemaVersion": 1,
                "id": "marketplace/css",
                "version": "1.0.0",
                "packageDigest": digest,
            }),
        );
        package_target.custom.insert(
            "marketplaceCatalog".into(),
            serde_json::json!({
                "schemaVersion": 1,
                "manifest": {
                    "schemaVersion": 1,
                    "packageType": "language",
                    "source": "official",
                    "id": "marketplace/css",
                    "version": "1.0.0",
                    "displayName": "CSS Language Support",
                    "description": "CSS language intelligence",
                    "license": "MIT",
                    "languages": [{
                        "id": "css",
                        "displayName": "CSS",
                        "aliases": ["css"],
                        "fileExtensions": [".css"],
                        "lsp": true
                    }],
                    "capabilities": [
                        {"kind":"asset","id":"language-assets","path":"language"},
                        {"kind":"executable","id":"css-language-server","path":"server/css-language-server"}
                    ]
                },
                "consumerMetadata": {},
                "packageFileCount": file_count,
                "packageSizeBytes": size_bytes,
            }),
        );
        let package_name = TargetName::new("packages/marketplace/css/1.0.0.zip").unwrap();
        let language_index_name = TargetName::new("marketplace/languages/index.json").unwrap();
        let revocations_name = TargetName::new("marketplace/revocations.json").unwrap();
        let one = NonZeroU64::new(1).unwrap();
        let later: Timestamp = "2999-01-01T00:00:00Z".parse().unwrap();
        let mut editor = RepositoryEditor::new(&trusted_root_path).await.unwrap();
        editor
            .snapshot_version(one)
            .snapshot_expires(later)
            .timestamp_version(one)
            .timestamp_expires(later)
            .add_target(
                language_index_name.clone(),
                Target::from_path(&language_index_path).await.unwrap(),
            )
            .unwrap()
            .add_target(
                revocations_name.clone(),
                Target::from_path(&revocations_path).await.unwrap(),
            )
            .unwrap()
            .delegate_role(
                "publishers/marketplace",
                &keys,
                PathSet::Paths(vec![
                    PathPattern::new("packages/marketplace/css/1.0.0.zip").unwrap(),
                ]),
                true,
                one,
                later,
                one,
            )
            .await
            .unwrap()
            .targets_version(one)
            .unwrap()
            .targets_expires(later)
            .unwrap()
            .sign_targets_editor(&keys)
            .await
            .unwrap()
            .change_delegated_targets("publishers/marketplace")
            .unwrap()
            .add_target(package_name.clone(), package_target)
            .unwrap()
            .targets_version(one)
            .unwrap()
            .targets_expires(later)
            .unwrap()
            .sign_targets_editor(&keys)
            .await
            .unwrap();
        editor
            .sign(&keys)
            .await
            .unwrap()
            .write(repository.path().join("metadata"))
            .await
            .unwrap();
        for (name, source) in [
            (&package_name, &archive_path),
            (&language_index_name, &language_index_path),
            (&revocations_name, &revocations_path),
        ] {
            write_target(repository.path(), name, fs::read(source).unwrap());
        }
        Self {
            trusted_root: fs::read(trusted_root_path).unwrap(),
            repository,
            cache,
        }
    }
}

async fn write_trusted_root(path: &Path, keys: &[Box<dyn KeySource>]) {
    let key = keys[0].as_sign().await.unwrap().tuf_key();
    let key_id = key.key_id().unwrap().clone();
    let role = RoleKeys {
        keyids: vec![key_id.clone()],
        threshold: NonZeroU64::new(1).unwrap(),
        _extra: HashMap::new(),
    };
    let later: Timestamp = "2999-01-01T00:00:00Z".parse().unwrap();
    let mut root = Root {
        spec_version: "1.0.0".into(),
        consistent_snapshot: false,
        version: NonZeroU64::new(1).unwrap(),
        expires: later,
        keys: HashMap::new(),
        roles: HashMap::from([
            (RoleType::Root, role.clone()),
            (RoleType::Snapshot, role.clone()),
            (RoleType::Targets, role.clone()),
            (RoleType::Timestamp, role),
        ]),
        _extra: HashMap::new(),
    };
    root.keys.insert(key_id, key);
    let signed = SignedRole::new(
        root.clone(),
        &KeyHolder::Root(root),
        keys,
        &SystemRandom::new(),
    )
    .await
    .unwrap();
    fs::write(path, signed.buffer()).unwrap();
}

fn write_css_package(root: &Path) {
    fs::create_dir_all(root.join("language")).unwrap();
    fs::create_dir_all(root.join("server")).unwrap();
    fs::write(root.join("README.md"), "# CSS\n").unwrap();
    fs::write(root.join("LICENSE"), "MIT\n").unwrap();
    fs::write(root.join("language/package.json"), "{}").unwrap();
    fs::write(
        root.join("server/css-language-server"),
        "#!/usr/bin/env node\nprocess.stdout.write('');\n",
    )
    .unwrap();
}

fn package_digest(root: &Path) -> (String, u64, u64) {
    let files = [
        "LICENSE",
        "README.md",
        "language/package.json",
        "server/css-language-server",
    ];
    let mut hasher = Sha256::new();
    hasher.update(b"marketplace-package-v1\0");
    let mut size = 0_u64;
    for relative in files {
        let bytes = fs::read(root.join(relative)).unwrap();
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        size += bytes.len() as u64;
    }
    (
        format!("sha256:{:x}", hasher.finalize()),
        files.len() as u64,
        size,
    )
}

fn write_archive(root: &Path, destination: &Path) {
    let mut archive = ZipWriter::new(fs::File::create(destination).unwrap());
    let options = SimpleFileOptions::default();
    for relative in [
        "LICENSE",
        "README.md",
        "language/package.json",
        "server/css-language-server",
    ] {
        archive.start_file(relative, options).unwrap();
        archive
            .write_all(&fs::read(root.join(relative)).unwrap())
            .unwrap();
    }
    archive.finish().unwrap();
}

fn write_target(repository: &Path, name: &TargetName, bytes: Vec<u8>) {
    let path = repository.join("targets").join(name.raw());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

struct FixtureHttpClient {
    root: PathBuf,
}

impl HttpClient for FixtureHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError> {
        let url = Url::parse(request.url())
            .map_err(|_| HttpClientError::InvalidRequest("invalid fixture URL".into()))?;
        let candidate = self.root.join(url.path().trim_start_matches('/'));
        match fs::read(candidate) {
            Ok(bytes) => Ok(HttpResponse::new(200, Vec::new(), bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(HttpResponse::new(404, Vec::new(), Vec::new()))
            }
            Err(_) => Err(HttpClientError::Transport("fixture read failed".into())),
        }
    }
}
