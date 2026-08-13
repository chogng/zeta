use std::fs;
use std::path::Path;

use tempfile::TempDir;
use zeta_config::LanguageServerConfig;
use zeta_config::LanguageServerId;
use zeta_config::LanguageServerModeConfig;
use zeta_config::LanguageServersConfig;
use zeta_language_server_catalog::CSS_LANGUAGE_SERVER_ID;
use zeta_language_server_catalog::CssLanguageServerProvider;
use zeta_language_server_catalog::LanguageServerProviderRegistry;
use zeta_language_server_catalog::ManagedNodeRuntime;
use zeta_language_server_distribution::LanguageServerInstaller;
use zeta_language_server_distribution::LanguageServerPackage;
use zeta_language_server_distribution::LanguageServerPackageFile;

use super::configured_provider_definitions;

#[test]
fn configured_provider_definitions_require_explicit_user_enablement() {
    let fixture = ProviderFixture::new();
    let registry = fixture.registry();
    assert!(
        configured_provider_definitions(
            &registry,
            &LanguageServersConfig::default(),
            fixture.workspace.path(),
        )
        .unwrap()
        .is_empty()
    );

    let configuration = configuration(LanguageServerModeConfig::Automatic, None);
    let definitions =
        configured_provider_definitions(&registry, &configuration, fixture.workspace.path())
            .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name().as_str(), CSS_LANGUAGE_SERVER_ID);
}

#[test]
fn configured_provider_definitions_preserve_native_override_semantics() {
    let fixture = ProviderFixture::new();
    let native = fixture.root.path().join("native-css-server");
    write_executable(&native);
    let definitions = configured_provider_definitions(
        &fixture.registry(),
        &configuration(LanguageServerModeConfig::Enabled, Some(native.clone())),
        fixture.workspace.path(),
    )
    .unwrap();
    let (_, command, _) = definitions.into_iter().next().unwrap().into_launch_parts();
    assert_eq!(
        command.program(),
        fs::canonicalize(native).unwrap().as_os_str()
    );
    assert!(command.arguments().is_empty());
}

fn configuration(
    mode: LanguageServerModeConfig,
    executable: Option<std::path::PathBuf>,
) -> LanguageServersConfig {
    LanguageServersConfig {
        servers: [(
            LanguageServerId::new(CSS_LANGUAGE_SERVER_ID).unwrap(),
            LanguageServerConfig { mode, executable },
        )]
        .into_iter()
        .collect(),
    }
}

struct ProviderFixture {
    root: TempDir,
    workspace: TempDir,
    node: std::path::PathBuf,
}

impl ProviderFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        let node = root.path().join("node");
        write_executable(&node);
        Self {
            root,
            workspace,
            node,
        }
    }

    fn registry(&self) -> LanguageServerProviderRegistry {
        let package = LanguageServerPackage::new(
            CSS_LANGUAGE_SERVER_ID,
            "0.1.0",
            "server/css-language-server",
            vec![
                LanguageServerPackageFile::executable(
                    "server/css-language-server",
                    b"// server".to_vec(),
                )
                .unwrap(),
                LanguageServerPackageFile::regular(
                    "server/runtime/node_modules/example/index.js",
                    b"module.exports = {};".to_vec(),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let digest = package.sha256();
        let installed = LanguageServerInstaller::new(self.root.path().join("languages"))
            .unwrap()
            .install_verified(package, digest)
            .unwrap();
        let provider = CssLanguageServerProvider::new(
            installed,
            ManagedNodeRuntime::from_path(&self.node).unwrap(),
        )
        .unwrap();
        let mut registry = LanguageServerProviderRegistry::new();
        registry.register(provider).unwrap();
        registry
    }
}

fn write_executable(path: &Path) {
    fs::write(path, b"runtime").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
