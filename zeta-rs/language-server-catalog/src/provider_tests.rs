use std::fs;
use std::path::Path;

use tempfile::TempDir;
use zeta_language_server_distribution::LanguageServerInstaller;
use zeta_language_server_distribution::LanguageServerPackage;
use zeta_language_server_distribution::LanguageServerPackageFile;
use zeta_lsp::LanguageServerEnvironmentPolicy;

use crate::CSS_LANGUAGE_SERVER_ID;
use crate::CssLanguageServerProvider;
use crate::LanguageServerProviderError;
use crate::LanguageServerProviderLaunch;
use crate::LanguageServerProviderRegistry;
use crate::ManagedNodeRuntime;

#[test]
fn css_provider_uses_managed_node_and_a_clean_environment() {
    let fixture = ProviderFixture::new();
    let provider = fixture.provider();
    let mut registry = LanguageServerProviderRegistry::new();
    registry.register(provider).unwrap();

    let definition = registry
        .definition(
            CSS_LANGUAGE_SERVER_ID,
            fixture.workspace.path(),
            LanguageServerProviderLaunch::Packaged,
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        definition.language_ids().collect::<Vec<_>>(),
        vec!["css", "less", "scss"]
    );
    let (_, command, _) = definition.into_launch_parts();
    let node = fs::canonicalize(&fixture.node).unwrap();
    let entrypoint = fs::canonicalize(&fixture.entrypoint).unwrap();
    assert_eq!(command.program(), node.as_os_str());
    assert_eq!(
        command.arguments(),
        [entrypoint.as_os_str(), std::ffi::OsStr::new("--stdio")]
    );
    assert_eq!(
        command.environment_policy(),
        LanguageServerEnvironmentPolicy::Clear
    );
    assert!(
        !command
            .environment()
            .contains_key(std::ffi::OsStr::new("NODE_OPTIONS"))
    );
    assert!(
        !command
            .environment()
            .contains_key(std::ffi::OsStr::new("NODE_PATH"))
    );
    assert_eq!(command.current_dir(), Some(fixture.workspace.path()));
}

#[test]
fn css_provider_accepts_an_authoritative_native_override() {
    let fixture = ProviderFixture::new();
    let provider = fixture.provider();
    let native = fixture.root.path().join("native-css-server");
    write_executable(&native);
    let mut registry = LanguageServerProviderRegistry::new();
    registry.register(provider).unwrap();

    let definition = registry
        .definition(
            CSS_LANGUAGE_SERVER_ID,
            fixture.workspace.path(),
            LanguageServerProviderLaunch::ExplicitExecutable(&native),
        )
        .unwrap()
        .unwrap();
    let (_, command, _) = definition.into_launch_parts();
    assert_eq!(
        command.program(),
        fs::canonicalize(native).unwrap().as_os_str()
    );
    assert!(command.arguments().is_empty());
    assert_eq!(
        command.environment_policy(),
        LanguageServerEnvironmentPolicy::Inherit
    );
}

#[test]
fn registry_rejects_duplicate_provider_identity() {
    let fixture = ProviderFixture::new();
    let mut registry = LanguageServerProviderRegistry::new();
    registry.register(fixture.provider()).unwrap();
    let error = registry.register(fixture.provider()).unwrap_err();
    assert!(matches!(
        error,
        LanguageServerProviderError::DuplicateProvider(CSS_LANGUAGE_SERVER_ID)
    ));
}

struct ProviderFixture {
    root: TempDir,
    workspace: TempDir,
    node: std::path::PathBuf,
    entrypoint: std::path::PathBuf,
}

impl ProviderFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        let node = root.path().join("node");
        let entrypoint = root
            .path()
            .join("languages")
            .join(CSS_LANGUAGE_SERVER_ID)
            .join("0.1.0")
            .join("server/css-language-server");
        write_executable(&node);
        Self {
            root,
            workspace,
            node,
            entrypoint,
        }
    }

    fn provider(&self) -> CssLanguageServerProvider {
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
        CssLanguageServerProvider::new(
            installed,
            ManagedNodeRuntime::from_path(&self.node).unwrap(),
        )
        .unwrap()
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
