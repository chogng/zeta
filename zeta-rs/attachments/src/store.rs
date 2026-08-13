use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use zeta_protocol::ContentDigest;
use zeta_protocol::ImageAttachmentRef;
use zeta_utils_image::EncodedImage;

use crate::AttachmentError;
use crate::service::reference_for_image;
use crate::service::verify_reference_bytes;

/// Stores and resolves immutable image bytes by their exact content digest.
///
/// Implementations must commit bytes before returning their reference and must verify untrusted
/// reference metadata rather than using it to select arbitrary filesystem paths.
pub trait ImageAttachmentStore: Send + Sync {
    fn put(&self, image: &EncodedImage) -> Result<ImageAttachmentRef, AttachmentError>;
    fn read(&self, reference: &ImageAttachmentRef) -> Result<Arc<[u8]>, AttachmentError>;
}

/// Process-local store used by tests and explicitly ephemeral products.
#[derive(Default)]
pub struct MemoryImageAttachmentStore {
    bytes: Mutex<BTreeMap<ContentDigest, Arc<[u8]>>>,
}

impl ImageAttachmentStore for MemoryImageAttachmentStore {
    fn put(&self, image: &EncodedImage) -> Result<ImageAttachmentRef, AttachmentError> {
        let reference = reference_for_image(image)?;
        self.bytes
            .lock()
            .map_err(|_| AttachmentError::Corrupt)?
            .insert(reference.content_digest.clone(), Arc::clone(&image.bytes));
        Ok(reference)
    }

    fn read(&self, reference: &ImageAttachmentRef) -> Result<Arc<[u8]>, AttachmentError> {
        self.bytes
            .lock()
            .map_err(|_| AttachmentError::Corrupt)?
            .get(&reference.content_digest)
            .cloned()
            .ok_or(AttachmentError::NotFound)
    }
}

/// Crash-safe content-addressed store rooted under one application profile.
pub struct FileImageAttachmentStore {
    root: PathBuf,
}

impl FileImageAttachmentStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, AttachmentError> {
        let root = root.into();
        create_private_directory(&root)?;
        Ok(Self { root })
    }

    fn path_for(&self, digest: &ContentDigest) -> PathBuf {
        let hex = digest
            .as_str()
            .strip_prefix("sha256:")
            .expect("ContentDigest always validates its algorithm prefix");
        self.root.join("sha256").join(&hex[..2]).join(hex)
    }
}

impl ImageAttachmentStore for FileImageAttachmentStore {
    fn put(&self, image: &EncodedImage) -> Result<ImageAttachmentRef, AttachmentError> {
        let reference = reference_for_image(image)?;
        let path = self.path_for(&reference.content_digest);
        if path.exists() {
            let bytes = read_regular_file(&path)?;
            verify_reference_bytes(&reference, &bytes)?;
            return Ok(reference);
        }
        let parent = path.parent().expect("attachment paths always have a parent");
        create_private_directory(parent)?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".attachment-")
            .tempfile_in(parent)
            .map_err(|source| AttachmentError::storage(parent, source))?;
        set_private_file_permissions(temporary.as_file(), temporary.path())?;
        temporary
            .write_all(&image.bytes)
            .map_err(|source| AttachmentError::storage(temporary.path(), source))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|source| AttachmentError::storage(temporary.path(), source))?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => sync_directory(parent)?,
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let bytes = read_regular_file(&path)?;
                verify_reference_bytes(&reference, &bytes)?;
            }
            Err(error) => return Err(AttachmentError::storage(&path, error.error)),
        }
        Ok(reference)
    }

    fn read(&self, reference: &ImageAttachmentRef) -> Result<Arc<[u8]>, AttachmentError> {
        let path = self.path_for(&reference.content_digest);
        let bytes = read_regular_file(&path)?;
        Ok(bytes.into())
    }
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>, AttachmentError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            AttachmentError::NotFound
        } else {
            AttachmentError::storage(path, source)
        }
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(AttachmentError::Corrupt);
    }
    fs::read(path).map_err(|source| AttachmentError::storage(path, source))
}

fn create_private_directory(path: &Path) -> Result<(), AttachmentError> {
    fs::create_dir_all(path).map_err(|source| AttachmentError::storage(path, source))?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| AttachmentError::storage(path, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(AttachmentError::Corrupt);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|source| AttachmentError::storage(path, source))?;
    }
    Ok(())
}

fn set_private_file_permissions(file: &fs::File, path: &Path) -> Result<(), AttachmentError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|source| AttachmentError::storage(path, source))?;
    }
    #[cfg(not(unix))]
    let _ = (file, path);
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), AttachmentError> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| AttachmentError::storage(path, source))
}
