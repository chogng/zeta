use crate::FileInformation;
use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;

pub(super) fn inspect(file: &File) -> io::Result<FileInformation> {
    let metadata = file.metadata()?;
    Ok(FileInformation::new(
        metadata.dev(),
        metadata.ino(),
        metadata.nlink(),
    ))
}
