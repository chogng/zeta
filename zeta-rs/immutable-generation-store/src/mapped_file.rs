use memmap2::Mmap;
use memmap2::MmapOptions;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct GenerationLease {
    pub(crate) _file: fs::File,
}

/// A mapped base-generation file that keeps its generation lease alive.
#[derive(Debug)]
pub struct MappedGenerationFile {
    mapping: Mmap,
    _file: fs::File,
    _lease: Arc<GenerationLease>,
}

impl MappedGenerationFile {
    pub(crate) fn open(file: fs::File, lease: Arc<GenerationLease>) -> io::Result<Self> {
        let length = file.metadata()?.len();
        if length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cannot map an empty generation file",
            ));
        }
        let length = usize::try_from(length).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "generation file length exceeds the address space",
            )
        })?;
        let mapping = map_generation_file(&file, length)?;
        if file.metadata()?.len() != length as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "generation file changed while it was being mapped",
            ));
        }
        Ok(Self {
            mapping,
            _file: file,
            _lease: lease,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.mapping
    }
}

#[allow(unsafe_code)]
fn map_generation_file(file: &fs::File, length: usize) -> io::Result<Mmap> {
    // SAFETY: Generation files are created under the store's exclusive lock, published under a
    // unique directory, and never modified in place. The shared generation lease prevents store
    // cleanup while the mapping exists. The retained file handle refers to the same generation.
    unsafe { MmapOptions::new().len(length).map(file) }
}

impl AsRef<[u8]> for MappedGenerationFile {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Deref for MappedGenerationFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// An open base-generation file that keeps its generation lease alive.
#[derive(Debug)]
pub struct OpenGenerationFile {
    file: fs::File,
    _lease: Arc<GenerationLease>,
}

impl OpenGenerationFile {
    pub(crate) fn new(file: fs::File, lease: Arc<GenerationLease>) -> Self {
        Self {
            file,
            _lease: lease,
        }
    }

    pub fn length(&self) -> io::Result<u64> {
        self.file.metadata().map(|metadata| metadata.len())
    }
}

impl Read for OpenGenerationFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file.read(buffer)
    }
}

impl Seek for OpenGenerationFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}
