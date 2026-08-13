use crate::InstalledPluginRef;
use crate::LocalPluginPackage;
use crate::PluginError;
use crate::PluginErrorKind;
use crate::PluginPackageDigest;
use crate::PluginPackageSource;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Immutable package object selected for one Plugin activation generation.
#[derive(Clone, Debug)]
pub struct InstalledPluginPackage {
    package: LocalPluginPackage,
    object_root: PathBuf,
}

impl InstalledPluginPackage {
    pub fn manifest(&self) -> &crate::PluginManifest {
        self.package.manifest()
    }

    pub fn package_digest(&self) -> &PluginPackageDigest {
        self.package.package_digest()
    }

    pub fn package(&self) -> &LocalPluginPackage {
        &self.package
    }

    /// Returns the root of this exact immutable package object for trusted runtime composition.
    ///
    /// Launchers may use this path to bind the extension process working directory or a sandbox
    /// mount. Contributions must continue to resolve declared files through [`Self::resolve_file`]
    /// or [`Self::resolve_directory`] and must not use this accessor to bypass path validation.
    pub fn package_root(&self) -> &Path {
        &self.object_root
    }

    /// Resolves one validated regular file inside this immutable object.
    pub fn resolve_file(&self, path: &crate::PluginPath) -> Result<PathBuf, PluginError> {
        let candidate = self.object_root.join(path.to_platform_path());
        let canonical = candidate.canonicalize().map_err(store_io)?;
        if !canonical.starts_with(&self.object_root)
            || !fs::symlink_metadata(&candidate)
                .map_err(store_io)?
                .is_file()
        {
            return Err(PluginError::new(
                PluginErrorKind::PackageUnsafe,
                "installed Plugin file escaped its immutable object",
            ));
        }
        Ok(canonical)
    }

    /// Resolves one validated directory inside this immutable object.
    pub fn resolve_directory(&self, path: &crate::PluginPath) -> Result<PathBuf, PluginError> {
        let candidate = self.object_root.join(path.to_platform_path());
        let canonical = candidate.canonicalize().map_err(store_io)?;
        if !canonical.starts_with(&self.object_root)
            || !fs::symlink_metadata(&candidate).map_err(store_io)?.is_dir()
        {
            return Err(PluginError::new(
                PluginErrorKind::PackageUnsafe,
                "installed Plugin directory escaped its immutable object",
            ));
        }
        Ok(canonical)
    }

    /// Reads one bounded UTF-8 definition file from this immutable object.
    pub fn read_utf8_file(
        &self,
        path: &crate::PluginPath,
        maximum_bytes: u64,
    ) -> Result<String, PluginError> {
        let file = self.resolve_file(path)?;
        let metadata = fs::metadata(&file).map_err(store_io)?;
        if metadata.len() > maximum_bytes {
            return Err(PluginError::new(
                PluginErrorKind::ContributionInvalid,
                "installed Plugin definition exceeds its size limit",
            ));
        }
        fs::read_to_string(file).map_err(|_| {
            PluginError::new(
                PluginErrorKind::ContributionInvalid,
                "installed Plugin definition is not valid UTF-8",
            )
        })
    }
}

/// Immutable content-addressed store for validated local Plugin packages.
///
/// Installation copies into a unique staging directory, validates the copy, and then atomically
/// promotes it. It does not enable contributions, grant permissions, or start a runtime.
#[derive(Clone, Debug)]
pub struct PluginPackageStore {
    root: PathBuf,
}

impl PluginPackageStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, PluginError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("objects")).map_err(store_io)?;
        fs::create_dir_all(root.join("staging")).map_err(store_io)?;
        let root = root.canonicalize().map_err(store_io)?;
        sync_directory(&root)?;
        Ok(Self { root })
    }

    /// Removes only transient staging leftovers from an interrupted installation.
    ///
    /// The object store deliberately retains unreferenced immutable packages: a caller may have
    /// installed a package into the store immediately before opening the authority and still be
    /// preparing its durable Install command.
    pub(crate) fn recover_orphans(&self) -> Result<(), PluginError> {
        let staging = self.root.join("staging");
        for entry in fs::read_dir(&staging).map_err(store_io)? {
            let entry = entry.map_err(store_io)?;
            let metadata = entry.file_type().map_err(store_io)?;
            if metadata.is_dir() {
                fs::remove_dir_all(entry.path()).map_err(store_io)?;
            } else {
                return Err(PluginError::new(
                    PluginErrorKind::PackageUnsafe,
                    "Plugin staging contains a non-directory orphan",
                ));
            }
        }
        sync_directory(&staging)?;
        sync_directory(&self.root)
    }

    pub fn install_local(
        &self,
        package: &LocalPluginPackage,
    ) -> Result<InstalledPluginRef, PluginError> {
        let PluginPackageSource::LocalDevelopment { canonical_path } = package.source() else {
            return Err(PluginError::new(
                PluginErrorKind::SourceUnavailable,
                "built-in Plugin installation is owned by the Zeta release",
            ));
        };
        let operation_id = new_operation_id()?;
        let staging = self.root.join("staging").join(&operation_id);
        let result = (|| {
            fs::create_dir(&staging).map_err(store_io)?;
            copy_package_tree(canonical_path, &staging)?;
            let installed = LocalPluginPackage::load(&staging)?;
            if installed.package_digest() != package.package_digest()
                || installed.manifest().id != package.manifest().id
                || installed.manifest().version != package.manifest().version
            {
                return Err(PluginError::new(
                    PluginErrorKind::PackageConflict,
                    "Plugin source changed while it was being installed",
                ));
            }
            let object = self.object_path(installed.package_digest());
            if object.exists() {
                let existing = LocalPluginPackage::load(&object)?;
                if existing.package_digest() != installed.package_digest() {
                    return Err(PluginError::new(
                        PluginErrorKind::PackageConflict,
                        "Plugin object path contains different content",
                    ));
                }
            } else {
                sync_directory_tree(&staging)?;
                fs::rename(&staging, &object).map_err(store_io)?;
                sync_directory(&self.root.join("objects"))?;
            }
            Ok(InstalledPluginRef {
                id: installed.manifest().id.clone(),
                version: installed.manifest().version.clone(),
                digest: installed.package_digest().clone(),
            })
        })();
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub fn read(&self, installed: &InstalledPluginRef) -> Result<LocalPluginPackage, PluginError> {
        let package = LocalPluginPackage::load(self.object_path(&installed.digest))?;
        if package.manifest().id != installed.id
            || package.manifest().version != installed.version
            || package.package_digest() != &installed.digest
        {
            return Err(PluginError::new(
                PluginErrorKind::PackageConflict,
                "installed Plugin reference does not match immutable object content",
            ));
        }
        Ok(package)
    }

    /// Loads one exact immutable package object for activation consumers.
    pub fn activate(
        &self,
        installed: &InstalledPluginRef,
    ) -> Result<InstalledPluginPackage, PluginError> {
        let package = self.read(installed)?;
        Ok(InstalledPluginPackage {
            package,
            object_root: self.object_path(&installed.digest),
        })
    }

    /// Removes one exact immutable object after its authority reference has been removed.
    pub(crate) fn remove_object(&self, digest: &PluginPackageDigest) -> Result<(), PluginError> {
        let object = self.object_path(digest);
        match fs::symlink_metadata(&object) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                fs::remove_dir_all(&object).map_err(store_io)?;
                sync_directory(&self.root.join("objects"))
            }
            Ok(_) => Err(PluginError::new(
                PluginErrorKind::PackageUnsafe,
                "Plugin object path is not a regular directory",
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(store_io(error)),
        }
    }

    fn object_path(&self, digest: &PluginPackageDigest) -> PathBuf {
        self.root.join("objects").join(
            digest
                .as_str()
                .strip_prefix("sha256:")
                .expect("validated digest has a SHA-256 prefix"),
        )
    }
}

fn new_operation_id() -> Result<String, PluginError> {
    let mut random = [0_u8; 16];
    getrandom::getrandom(&mut random).map_err(|_| {
        PluginError::new(
            PluginErrorKind::SourceUnavailable,
            "Plugin staging identity could not be generated",
        )
    })?;
    let mut value = String::with_capacity(random.len() * 2);
    for byte in random {
        write!(value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(value)
}

fn copy_package_tree(source: &Path, target: &Path) -> Result<(), PluginError> {
    let mut entries = fs::read_dir(source)
        .map_err(store_io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(store_io)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let metadata = entry.file_type().map_err(store_io)?;
        let destination = target.join(entry.file_name());
        let inspected = fs::symlink_metadata(entry.path()).map_err(store_io)?;
        if metadata.is_symlink()
            || inspected.file_type().is_symlink()
            || (!metadata.is_dir() && !metadata.is_file())
        {
            return Err(PluginError::new(
                PluginErrorKind::PackageUnsafe,
                "Plugin package changed to contain an unsafe entry during installation",
            ));
        }
        if metadata.is_dir() {
            fs::create_dir(&destination).map_err(store_io)?;
            copy_package_tree(&entry.path(), &destination)?;
        } else {
            copy_file(&entry.path(), &destination, &inspected)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, target: &Path, inspected: &fs::Metadata) -> Result<(), PluginError> {
    let mut input = fs::File::open(source).map_err(store_io)?;
    let opened = input.metadata().map_err(store_io)?;
    if !opened.is_file() || !same_file_identity(inspected, &opened) {
        return Err(PluginError::new(
            PluginErrorKind::PackageUnsafe,
            "Plugin package entry changed while it was being installed",
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options.open(target).map_err(store_io)?;
    std::io::copy(&mut input, &mut output).map_err(store_io)?;
    output.sync_all().map_err(store_io)
}

#[cfg(unix)]
fn same_file_identity(inspected: &fs::Metadata, opened: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    inspected.dev() == opened.dev() && inspected.ino() == opened.ino()
}

#[cfg(not(unix))]
fn same_file_identity(inspected: &fs::Metadata, opened: &fs::Metadata) -> bool {
    inspected.len() == opened.len()
        && inspected.modified().ok() == opened.modified().ok()
        && inspected.file_type() == opened.file_type()
}

fn sync_directory_tree(path: &Path) -> Result<(), PluginError> {
    let mut entries = fs::read_dir(path)
        .map_err(store_io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(store_io)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if entry.file_type().map_err(store_io)?.is_dir() {
            sync_directory_tree(&entry.path())?;
        }
    }
    sync_directory(path)
}

fn sync_directory(path: &Path) -> Result<(), PluginError> {
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(store_io)?;
    Ok(())
}

fn store_io(_: impl std::fmt::Display) -> PluginError {
    PluginError::new(
        PluginErrorKind::SourceUnavailable,
        "Plugin package store operation failed",
    )
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
