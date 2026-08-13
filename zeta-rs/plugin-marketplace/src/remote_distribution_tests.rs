use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::Ed25519KeyPair;
use jiff::Timestamp;
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
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginActivationAuthority;
use zeta_plugins::PluginAuthorityCommandId;
use zeta_plugins::PluginMarketplaceId;
use zeta_plugins::PluginMarketplaceMode;
use zeta_plugins::PluginMarketplaceService;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::RemotePluginMarketplace;
use crate::RemotePluginMarketplaceConfig;

#[test]
fn signed_catalog_defers_package_download_and_revalidates_on_demand_cache() {
    let fixture = SignedDistributionFixture::create();
    let client = Arc::new(FixtureHttpClient {
        root: fixture.repository.path().to_path_buf(),
        offline: AtomicBool::new(false),
    });
    let marketplace = RemotePluginMarketplace::new(
        RemotePluginMarketplaceConfig::new(
            PluginMarketplaceId::new("zeta-test").unwrap(),
            Url::parse("https://marketplace.example/metadata/").unwrap(),
            Url::parse("https://marketplace.example/targets/").unwrap(),
            fixture.trusted_root,
            fixture.cache.path(),
        )
        .unwrap(),
        client.clone(),
    );

    let online = marketplace.sync().unwrap();
    assert_eq!(
        online.marketplace().mode(),
        PluginMarketplaceMode::RemoteManaged
    );
    assert_eq!(
        online.marketplace().list()[0].package_ref(),
        fixture.package
    );
    assert!(!fixture.cache.path().join("packages").exists());
    assert!(
        find_file_with_extension(&fixture.cache.path().join("repository/targets"), "zip").is_none()
    );

    client.offline.store(true, Ordering::Release);
    let offline = marketplace.sync().unwrap();
    assert_eq!(
        offline.marketplace().list()[0].package_ref(),
        fixture.package
    );

    client.offline.store(false, Ordering::Release);
    let authority = PluginActivationAuthority::open(fixture.cache.path().join("profile")).unwrap();
    let service = PluginMarketplaceService::new(authority, [offline.into_marketplace()]).unwrap();
    service
        .install(
            PluginAuthorityCommandId::new("install-on-demand").unwrap(),
            0,
            &PluginMarketplaceId::new("zeta-test").unwrap(),
            &fixture.package,
        )
        .unwrap();
    let cached_package = fixture.cache.path().join("packages").join(
        fixture
            .package
            .digest
            .as_str()
            .trim_start_matches("sha256:"),
    );
    assert!(cached_package.join(".zeta-plugin/plugin.json").is_file());

    fs::write(cached_package.join("skills/review/SKILL.md"), b"tampered").unwrap();
    client.offline.store(true, Ordering::Release);
    let cached = marketplace.open_cached().unwrap();
    let authority =
        PluginActivationAuthority::open(fixture.cache.path().join("tamper-profile")).unwrap();
    let service = PluginMarketplaceService::new(authority, [cached.into_marketplace()]).unwrap();
    let error = service
        .install(
            PluginAuthorityCommandId::new("install-tampered").unwrap(),
            0,
            &PluginMarketplaceId::new("zeta-test").unwrap(),
            &fixture.package,
        )
        .unwrap_err();
    assert_eq!(error.kind(), zeta_plugins::PluginErrorKind::PackageUnsafe);
}

struct SignedDistributionFixture {
    repository: TempDir,
    cache: TempDir,
    trusted_root: Vec<u8>,
    package: zeta_plugins::InstalledPluginRef,
}

impl SignedDistributionFixture {
    fn create() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(Self::create_async())
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
        let archive_path = repository.path().join("review.zip");
        write_plugin_package(&package_root);
        write_archive(&package_root, &archive_path);
        let package = LocalPluginPackage::load(&package_root).unwrap();
        let package_ref = zeta_plugins::InstalledPluginRef {
            id: package.manifest().id.clone(),
            version: package.manifest().version.clone(),
            digest: package.package_digest().clone(),
        };

        let revocations_path = repository.path().join("revocations.json");
        fs::write(&revocations_path, br#"{"schemaVersion":1,"revoked":[]}"#).unwrap();
        let mut package_target = Target::from_path(&archive_path).await.unwrap();
        package_target.custom.insert(
            "zetaPlugin".into(),
            serde_json::json!({
                "schemaVersion": 1,
                "id": package_ref.id,
                "version": package_ref.version,
                "packageDigest": package_ref.digest,
            }),
        );
        package_target.custom.insert(
            "zetaCatalog".into(),
            serde_json::json!({
                "schemaVersion": 1,
                "manifest": package.manifest(),
                "packageFileCount": package.stats().file_count,
                "packageSizeBytes": package.stats().total_bytes,
            }),
        );
        let revocations_target = Target::from_path(&revocations_path).await.unwrap();
        let package_name = TargetName::new("plugins/acme/review/1.0.0.zip").unwrap();
        let revocations_name = TargetName::new("zeta/revocations.json").unwrap();
        let one = NonZeroU64::new(1).unwrap();
        let later: Timestamp = "2999-01-01T00:00:00Z".parse().unwrap();
        let mut editor = RepositoryEditor::new(&trusted_root_path).await.unwrap();
        editor
            .snapshot_version(one)
            .snapshot_expires(later)
            .timestamp_version(one)
            .timestamp_expires(later)
            .add_target(revocations_name.clone(), revocations_target)
            .unwrap()
            .delegate_role(
                "publishers/acme",
                &keys,
                PathSet::Paths(vec![
                    PathPattern::new("plugins/acme/review/1.0.0.zip").unwrap(),
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
            .change_delegated_targets("publishers/acme")
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
        let signed = editor.sign(&keys).await.unwrap();
        signed
            .write(repository.path().join("metadata"))
            .await
            .unwrap();
        write_target(
            repository.path(),
            &package_name,
            fs::read(archive_path).unwrap(),
        );
        write_target(
            repository.path(),
            &revocations_name,
            fs::read(revocations_path).unwrap(),
        );

        Self {
            trusted_root: fs::read(trusted_root_path).unwrap(),
            repository,
            cache,
            package: package_ref,
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

fn write_plugin_package(root: &Path) {
    fs::create_dir_all(root.join(".zeta-plugin")).unwrap();
    fs::create_dir_all(root.join("skills/review")).unwrap();
    fs::write(root.join("skills/review/SKILL.md"), "# Review\n").unwrap();
    fs::write(
        root.join(".zeta-plugin/plugin.json"),
        r#"{
          "schemaVersion": 1,
          "id": "acme/review",
          "version": "1.0.0",
          "displayName": "Acme Review",
          "compatibility": {"zeta": ">=0.1.0"},
          "contributions": {"skills": [{"id": "review", "path": "skills/review"}]}
        }"#,
    )
    .unwrap();
}

fn write_archive(root: &Path, destination: &Path) {
    let output = fs::File::create(destination).unwrap();
    let mut archive = ZipWriter::new(output);
    let options = SimpleFileOptions::default();
    for relative in [".zeta-plugin/plugin.json", "skills/review/SKILL.md"] {
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

fn find_file_with_extension(root: &Path, extension: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                directories.push(entry.path());
            } else if entry.path().extension().and_then(|value| value.to_str()) == Some(extension) {
                return Some(entry.path());
            }
        }
    }
    None
}

struct FixtureHttpClient {
    root: PathBuf,
    offline: AtomicBool,
}

impl HttpClient for FixtureHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError> {
        if self.offline.load(Ordering::Acquire) {
            return Err(HttpClientError::Transport("fixture offline".into()));
        }
        let url = Url::parse(request.url())
            .map_err(|_| HttpClientError::InvalidRequest("invalid fixture URL".into()))?;
        let path = url.path().trim_start_matches('/');
        let path = path.replace("%2F", "%2f");
        let candidate = self.root.join(path);
        match fs::read(candidate) {
            Ok(bytes) => Ok(HttpResponse::new(200, Vec::new(), bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(HttpResponse::new(404, Vec::new(), Vec::new()))
            }
            Err(_) => Err(HttpClientError::Transport("fixture read failed".into())),
        }
    }
}
