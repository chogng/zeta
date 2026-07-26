//! PDF document ingestion primitives backed by a bundled PDFium runtime.
//!
//! This crate owns the native PDF parsing boundary. It deliberately exposes
//! extracted page text with stable, one-based page numbers, but does not own
//! chunking, OCR, embeddings, retrieval, or agent memory.

use pdfium_render::prelude::{Pdfium, PdfiumError};
use std::fmt;
use std::path::{Path, PathBuf};

/// The location of a PDFium dynamic library supplied by a Zeta release.
///
/// Release staging places the library below `resources/native/pdfium`; callers
/// create this type from that PDFium root instead of relying on a system-wide
/// library search path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfiumRuntime {
    library_path: PathBuf,
}

impl PdfiumRuntime {
    /// Resolves the platform-specific PDFium library below a release-staged root.
    pub fn from_bundled_root(root: impl AsRef<Path>) -> Self {
        Self {
            library_path: root.as_ref().join(platform_library_relative_path()),
        }
    }

    /// Returns the exact dynamic-library path that will be loaded.
    pub fn library_path(&self) -> &Path {
        &self.library_path
    }
}

/// Text extracted from one PDF page.
///
/// `page_number` is one-based so it can be presented directly in citations and
/// used by the desktop PDF viewer without an index conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedPdfPage {
    pub page_number: u32,
    pub text: String,
}

/// The text-bearing pages produced by a native PDF extraction pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedPdfDocument {
    pub pages: Vec<ExtractedPdfPage>,
}

impl ExtractedPdfDocument {
    /// Returns the number of pages in the source document.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

/// Failures raised while loading PDFium or extracting a local PDF document.
#[derive(Debug)]
pub enum DocumentError {
    PdfiumLibraryMissing { path: PathBuf },
    Pdfium(PdfiumError),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PdfiumLibraryMissing { path } => {
                write!(
                    formatter,
                    "bundled PDFium library is missing: {}",
                    path.display()
                )
            }
            Self::Pdfium(error) => write!(formatter, "PDFium operation failed: {error}"),
        }
    }
}

impl std::error::Error for DocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PdfiumLibraryMissing { .. } => None,
            Self::Pdfium(error) => Some(error),
        }
    }
}

/// A process-local PDFium binding used to extract text from PDF documents.
///
/// PDFium is initialized once per process by `pdfium-render`. Create this type
/// during app-server startup and reuse it for every ingestion request rather
/// than attempting to bind the library for each document.
pub struct PdfTextExtractor {
    pdfium: Pdfium,
}

impl PdfTextExtractor {
    /// Binds the PDFium dynamic library supplied by `runtime`.
    pub fn bind(runtime: PdfiumRuntime) -> Result<Self, DocumentError> {
        if !runtime.library_path.is_file() {
            return Err(DocumentError::PdfiumLibraryMissing {
                path: runtime.library_path,
            });
        }
        let bindings =
            Pdfium::bind_to_library(&runtime.library_path).map_err(DocumentError::Pdfium)?;
        Ok(Self {
            pdfium: Pdfium::new(bindings),
        })
    }

    /// Extracts native text from every page of `path` in page order.
    ///
    /// Blank text is retained for image-only pages so a later OCR stage can
    /// decide which pages require rendering and recognition.
    pub fn extract_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ExtractedPdfDocument, DocumentError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path.as_ref(), None)
            .map_err(DocumentError::Pdfium)?;
        let pages = document
            .pages()
            .iter()
            .enumerate()
            .map(|(index, page)| {
                let page_number = u32::try_from(index + 1)
                    .expect("PDFium page count exceeds Zeta's supported u32 page numbering");
                let text = page.text().map_err(DocumentError::Pdfium)?.all();
                Ok(ExtractedPdfPage { page_number, text })
            })
            .collect::<Result<Vec<_>, DocumentError>>()?;
        Ok(ExtractedPdfDocument { pages })
    }
}

fn platform_library_relative_path() -> &'static Path {
    #[cfg(target_os = "macos")]
    return Path::new("lib/libpdfium.dylib");
    #[cfg(target_os = "windows")]
    return Path::new("bin/pdfium.dll");
    #[cfg(target_os = "linux")]
    return Path::new("lib/libpdfium.so");
    #[allow(unreachable_code)]
    Path::new("unsupported-platform")
}

#[cfg(test)]
#[path = "pdfium_tests.rs"]
mod tests;
