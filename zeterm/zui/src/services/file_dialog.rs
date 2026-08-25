use std::path::PathBuf;
use std::sync::Arc;

use super::SystemServiceError;

/// Named extension filter shown by a native file dialog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileDialogFilter {
    name: String,
    extensions: Vec<String>,
}

impl FileDialogFilter {
    /// Creates a named filter from extension strings without leading dots.
    pub fn new(
        name: impl Into<String>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }
}

/// Backend-independent options shared by open, folder, and save dialogs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileDialogOptions {
    title: Option<String>,
    initial_directory: Option<PathBuf>,
    suggested_file_name: Option<String>,
    filters: Vec<FileDialogFilter>,
}

impl FileDialogOptions {
    /// Creates empty options using platform defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the native dialog title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the directory shown when the dialog opens.
    pub fn with_initial_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.initial_directory = Some(directory.into());
        self
    }

    /// Sets the initial file name used by save dialogs.
    pub fn with_suggested_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.suggested_file_name = Some(file_name.into());
        self
    }

    /// Appends one selectable extension filter.
    pub fn with_filter(mut self, filter: FileDialogFilter) -> Self {
        self.filters.push(filter);
        self
    }
}

/// Native file-dialog backend used through an injectable [`FileDialogHandle`].
pub trait FileDialogService: Send + Sync {
    /// Selects one file, or returns `None` when the user cancels.
    fn open_file(&self, options: FileDialogOptions) -> Result<Option<PathBuf>, SystemServiceError>;

    /// Selects zero or more files.
    fn open_files(&self, options: FileDialogOptions) -> Result<Vec<PathBuf>, SystemServiceError>;

    /// Selects one directory, or returns `None` when the user cancels.
    fn select_folder(
        &self,
        options: FileDialogOptions,
    ) -> Result<Option<PathBuf>, SystemServiceError>;

    /// Selects a destination path, or returns `None` when the user cancels.
    fn save_file(&self, options: FileDialogOptions) -> Result<Option<PathBuf>, SystemServiceError>;
}

/// Cloneable capability for opening native file, folder, and save dialogs.
#[derive(Clone)]
pub struct FileDialogHandle {
    service: Arc<dyn FileDialogService>,
}

impl FileDialogHandle {
    pub(crate) fn new(service: impl FileDialogService + 'static) -> Self {
        Self {
            service: Arc::new(service),
        }
    }

    /// Selects one file through the injected backend.
    pub fn open_file(
        &self,
        options: FileDialogOptions,
    ) -> Result<Option<PathBuf>, SystemServiceError> {
        self.service.open_file(options)
    }

    /// Selects zero or more files through the injected backend.
    pub fn open_files(
        &self,
        options: FileDialogOptions,
    ) -> Result<Vec<PathBuf>, SystemServiceError> {
        self.service.open_files(options)
    }

    /// Selects one directory through the injected backend.
    pub fn select_folder(
        &self,
        options: FileDialogOptions,
    ) -> Result<Option<PathBuf>, SystemServiceError> {
        self.service.select_folder(options)
    }

    /// Selects a destination path through the injected backend.
    pub fn save_file(
        &self,
        options: FileDialogOptions,
    ) -> Result<Option<PathBuf>, SystemServiceError> {
        self.service.save_file(options)
    }
}

/// Default native file-dialog backend.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemFileDialogs;

impl FileDialogService for SystemFileDialogs {
    fn open_file(&self, options: FileDialogOptions) -> Result<Option<PathBuf>, SystemServiceError> {
        Ok(dialog(options).pick_file())
    }

    fn open_files(&self, options: FileDialogOptions) -> Result<Vec<PathBuf>, SystemServiceError> {
        Ok(dialog(options).pick_files().unwrap_or_default())
    }

    fn select_folder(
        &self,
        options: FileDialogOptions,
    ) -> Result<Option<PathBuf>, SystemServiceError> {
        Ok(dialog(options).pick_folder())
    }

    fn save_file(&self, options: FileDialogOptions) -> Result<Option<PathBuf>, SystemServiceError> {
        Ok(dialog(options).save_file())
    }
}

fn dialog(options: FileDialogOptions) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new();
    if let Some(title) = options.title {
        dialog = dialog.set_title(title);
    }
    if let Some(directory) = options.initial_directory {
        dialog = dialog.set_directory(directory);
    }
    if let Some(file_name) = options.suggested_file_name {
        dialog = dialog.set_file_name(file_name);
    }
    for filter in options.filters {
        dialog = dialog.add_filter(filter.name, &filter.extensions);
    }
    dialog
}
