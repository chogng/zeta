use crate::layout::generation_name;
use crate::layout::parse_generation_name;
use crate::layout::parse_manifest_name;
use crate::mapped_file::GenerationLease;
use crate::mapped_file::MappedGenerationFile;
use crate::mapped_file::OpenGenerationFile;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

const BASES_DIRECTORY: &str = "bases";
const LAYERS_DIRECTORY: &str = "layers";
const LEASE_FILE: &str = ".lease";
const LOCK_FILE: &str = "store.lock";
const MANIFESTS_DIRECTORY: &str = "manifests";
const MANIFEST_VERSION: &[u8] = b"zeta-immutable-generation-v1\0";

/// A named byte file to publish inside an immutable base or change layer.
#[derive(Clone, Copy)]
pub struct GenerationFile<'a> {
    name: &'a str,
    contents: &'a [u8],
}

impl<'a> GenerationFile<'a> {
    pub fn new(name: &'a str, contents: &'a [u8]) -> Self {
        Self { name, contents }
    }
}

/// Owns immutable base generations, change-layer snapshots, manifests and process coordination.
#[derive(Clone, Debug)]
pub struct ImmutableGenerationStore {
    root: PathBuf,
}

impl ImmutableGenerationStore {
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let store = Self { root: root.into() };
        fs::create_dir_all(store.base_root())?;
        fs::create_dir_all(store.layer_root())?;
        fs::create_dir_all(store.manifest_root())?;
        store.open_lock()?;
        Ok(store)
    }

    pub fn open_current(&self) -> io::Result<Option<PublishedSnapshot>> {
        let lock = self.lock_shared()?;
        let Some(manifest) = self.latest_manifest()? else {
            return Ok(None);
        };
        let snapshot = self.open_snapshot(manifest)?;
        drop(lock);
        Ok(Some(snapshot))
    }

    pub fn publish_base(
        &self,
        snapshot: u64,
        base_files: &[GenerationFile<'_>],
        layer_files: &[GenerationFile<'_>],
    ) -> io::Result<()> {
        let lock = self.lock_exclusive()?;
        self.ensure_new_snapshot(snapshot)?;
        validate_files(base_files)?;
        validate_files(layer_files)?;
        self.remove_pending(snapshot)?;
        self.remove_unpublished_generation(&self.base_directory(snapshot))?;
        self.remove_unpublished_generation(&self.layer_directory(snapshot))?;
        let base = self.write_generation_directory(self.pending_base(snapshot), base_files)?;
        let layer = self.write_generation_directory(self.pending_layer(snapshot), layer_files)?;
        let base_target = self.base_directory(snapshot);
        let layer_target = self.layer_directory(snapshot);
        fs::rename(&base, &base_target)?;
        fs::rename(&layer, &layer_target)?;
        sync_directory(&self.base_root())?;
        sync_directory(&self.layer_root())?;
        self.publish_manifest(Manifest {
            snapshot,
            base: snapshot,
        })?;
        let _ = self.cleanup_stale(Manifest {
            snapshot,
            base: snapshot,
        });
        drop(lock);
        Ok(())
    }

    pub fn publish_layer(
        &self,
        snapshot: u64,
        layer_files: &[GenerationFile<'_>],
    ) -> io::Result<()> {
        let lock = self.lock_exclusive()?;
        let current = self.latest_manifest()?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no base generation is published")
        })?;
        if snapshot <= current.snapshot {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "snapshot generation must increase",
            ));
        }
        validate_files(layer_files)?;
        self.remove_pending(snapshot)?;
        self.remove_unpublished_generation(&self.layer_directory(snapshot))?;
        let layer = self.write_generation_directory(self.pending_layer(snapshot), layer_files)?;
        fs::rename(&layer, self.layer_directory(snapshot))?;
        sync_directory(&self.layer_root())?;
        let next = Manifest {
            snapshot,
            base: current.base,
        };
        self.publish_manifest(next)?;
        let _ = self.cleanup_stale(next);
        drop(lock);
        Ok(())
    }

    fn open_snapshot(&self, manifest: Manifest) -> io::Result<PublishedSnapshot> {
        let base_directory = self.base_directory(manifest.base);
        let layer_directory = self.layer_directory(manifest.snapshot);
        let base_lease = lock_generation(&base_directory)?;
        let layer_lease = lock_generation(&layer_directory)?;
        Ok(PublishedSnapshot {
            snapshot: manifest.snapshot,
            base: manifest.base,
            base_directory,
            layer_directory,
            base_lease,
            _layer_lease: layer_lease,
        })
    }

    fn ensure_new_snapshot(&self, snapshot: u64) -> io::Result<()> {
        if self
            .latest_manifest()?
            .is_some_and(|current| snapshot <= current.snapshot)
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "snapshot generation must increase",
            ));
        }
        Ok(())
    }

    fn write_generation_directory(
        &self,
        directory: PathBuf,
        files: &[GenerationFile<'_>],
    ) -> io::Result<PathBuf> {
        fs::create_dir(&directory)?;
        write_synced_file(&directory.join(LEASE_FILE), &[])?;
        for file in files {
            write_synced_file(&directory.join(file.name), file.contents)?;
        }
        sync_directory(&directory)?;
        Ok(directory)
    }

    fn publish_manifest(&self, manifest: Manifest) -> io::Result<()> {
        let pending = self.pending_manifest(manifest.snapshot);
        let target = self.manifest_path(manifest.snapshot);
        let mut contents = Vec::with_capacity(MANIFEST_VERSION.len() + 16);
        contents.extend_from_slice(MANIFEST_VERSION);
        contents.extend_from_slice(&manifest.snapshot.to_le_bytes());
        contents.extend_from_slice(&manifest.base.to_le_bytes());
        write_synced_file(&pending, &contents)?;
        fs::rename(pending, target)?;
        sync_directory(&self.manifest_root())
    }

    fn latest_manifest(&self) -> io::Result<Option<Manifest>> {
        let mut manifests = self.manifests()?;
        let Some((snapshot, path)) = manifests.pop_last() else {
            return Ok(None);
        };
        read_manifest(&path, snapshot).map(Some)
    }

    fn manifests(&self) -> io::Result<BTreeMap<u64, PathBuf>> {
        let mut manifests = BTreeMap::new();
        for entry in fs::read_dir(self.manifest_root())? {
            let entry = entry?;
            let Some(snapshot) = parse_manifest_name(&entry.file_name()) else {
                continue;
            };
            manifests.insert(snapshot, entry.path());
        }
        Ok(manifests)
    }

    fn cleanup_stale(&self, current: Manifest) -> io::Result<()> {
        let manifests = self.manifests()?;
        let mut retained_bases = BTreeSet::from([current.base]);
        for (snapshot, path) in manifests {
            if snapshot == current.snapshot {
                continue;
            }
            let manifest = read_manifest(&path, snapshot)?;
            let layer = self.layer_directory(snapshot);
            if generation_is_idle(&layer)? {
                fs::remove_dir_all(&layer)?;
                fs::remove_file(path)?;
            } else {
                retained_bases.insert(manifest.base);
            }
        }
        for entry in fs::read_dir(self.base_root())? {
            let entry = entry?;
            let Some(base) = parse_generation_name(&entry.file_name()) else {
                continue;
            };
            if !retained_bases.contains(&base) && generation_is_idle(&entry.path())? {
                fs::remove_dir_all(entry.path())?;
            }
        }
        sync_directory(&self.manifest_root())?;
        sync_directory(&self.layer_root())?;
        sync_directory(&self.base_root())
    }

    fn remove_pending(&self, snapshot: u64) -> io::Result<()> {
        remove_directory_if_present(&self.pending_base(snapshot))?;
        remove_directory_if_present(&self.pending_layer(snapshot))?;
        remove_file_if_present(&self.pending_manifest(snapshot))
    }

    fn remove_unpublished_generation(&self, directory: &Path) -> io::Result<()> {
        if !directory.exists() {
            return Ok(());
        }
        if !generation_is_idle(directory)? {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "unpublished generation is still leased",
            ));
        }
        fs::remove_dir_all(directory)
    }

    fn open_lock(&self) -> io::Result<fs::File> {
        fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join(LOCK_FILE))
    }

    fn lock_shared(&self) -> io::Result<fs::File> {
        let file = self.open_lock()?;
        fs2::FileExt::lock_shared(&file)?;
        Ok(file)
    }

    fn lock_exclusive(&self) -> io::Result<fs::File> {
        let file = self.open_lock()?;
        fs2::FileExt::lock_exclusive(&file)?;
        Ok(file)
    }

    fn base_root(&self) -> PathBuf {
        self.root.join(BASES_DIRECTORY)
    }

    fn layer_root(&self) -> PathBuf {
        self.root.join(LAYERS_DIRECTORY)
    }

    fn manifest_root(&self) -> PathBuf {
        self.root.join(MANIFESTS_DIRECTORY)
    }

    fn base_directory(&self, generation: u64) -> PathBuf {
        self.base_root().join(generation_name(generation))
    }

    fn layer_directory(&self, generation: u64) -> PathBuf {
        self.layer_root().join(generation_name(generation))
    }

    fn pending_base(&self, generation: u64) -> PathBuf {
        self.base_root()
            .join(format!(".pending-{}", generation_name(generation)))
    }

    fn pending_layer(&self, generation: u64) -> PathBuf {
        self.layer_root()
            .join(format!(".pending-{}", generation_name(generation)))
    }

    fn manifest_path(&self, generation: u64) -> PathBuf {
        self.manifest_root()
            .join(format!("{}.manifest", generation_name(generation)))
    }

    fn pending_manifest(&self, generation: u64) -> PathBuf {
        self.manifest_root()
            .join(format!(".pending-{}.manifest", generation_name(generation)))
    }
}

/// A consistent base generation and change-layer snapshot selected under the store lock.
pub struct PublishedSnapshot {
    snapshot: u64,
    base: u64,
    base_directory: PathBuf,
    layer_directory: PathBuf,
    base_lease: Arc<GenerationLease>,
    _layer_lease: Arc<GenerationLease>,
}

impl PublishedSnapshot {
    pub fn generation(&self) -> u64 {
        self.snapshot
    }

    pub fn base_generation(&self) -> u64 {
        self.base
    }

    pub fn read_base(&self, name: &str) -> io::Result<Vec<u8>> {
        validate_name(name)?;
        fs::read(self.base_directory.join(name))
    }

    pub fn read_layer(&self, name: &str) -> io::Result<Vec<u8>> {
        validate_name(name)?;
        fs::read(self.layer_directory.join(name))
    }

    pub fn map_base(&self, name: &str) -> io::Result<MappedGenerationFile> {
        validate_name(name)?;
        let file = fs::File::open(self.base_directory.join(name))?;
        MappedGenerationFile::open(file, Arc::clone(&self.base_lease))
    }

    pub fn open_base(&self, name: &str) -> io::Result<OpenGenerationFile> {
        validate_name(name)?;
        let file = fs::File::open(self.base_directory.join(name))?;
        Ok(OpenGenerationFile::new(file, Arc::clone(&self.base_lease)))
    }
}

#[derive(Clone, Copy)]
struct Manifest {
    snapshot: u64,
    base: u64,
}

fn lock_generation(directory: &Path) -> io::Result<Arc<GenerationLease>> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.join(LEASE_FILE))?;
    fs2::FileExt::lock_shared(&file)?;
    Ok(Arc::new(GenerationLease { _file: file }))
}

fn generation_is_idle(directory: &Path) -> io::Result<bool> {
    let lease_path = directory.join(LEASE_FILE);
    let file = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lease_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error),
    };
    match fs2::FileExt::try_lock_exclusive(&file) {
        Ok(()) => {
            fs2::FileExt::unlock(&file)?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(false),
        Err(error) => Err(error),
    }
}

fn validate_files(files: &[GenerationFile<'_>]) -> io::Result<()> {
    let mut names = BTreeSet::new();
    for file in files {
        validate_name(file.name)?;
        if !names.insert(file.name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "generation file names must be unique",
            ));
        }
    }
    Ok(())
}

fn validate_name(name: &str) -> io::Result<()> {
    let mut components = Path::new(name).components();
    if name == LEASE_FILE
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "generation file name must be one normal path component",
        ));
    }
    Ok(())
}

fn read_manifest(path: &Path, expected_snapshot: u64) -> io::Result<Manifest> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.len() != MANIFEST_VERSION.len() + 16 || !bytes.starts_with(MANIFEST_VERSION) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "generation manifest is corrupt",
        ));
    }
    let snapshot = read_u64(&bytes, MANIFEST_VERSION.len()).ok_or_else(corrupt_manifest)?;
    let base = read_u64(&bytes, MANIFEST_VERSION.len() + 8).ok_or_else(corrupt_manifest)?;
    if snapshot != expected_snapshot || base > snapshot {
        return Err(corrupt_manifest());
    }
    Ok(Manifest { snapshot, base })
}

fn corrupt_manifest() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "generation manifest is corrupt")
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn write_synced_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    fs::File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn remove_directory_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_file_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
