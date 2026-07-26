use super::*;
use std::env;
use std::path::PathBuf;

#[test]
fn bundled_runtime_uses_the_release_staging_layout() {
    let runtime = PdfiumRuntime::from_bundled_root("/release/resources/native/pdfium");
    let expected = if cfg!(target_os = "macos") {
        "/release/resources/native/pdfium/lib/libpdfium.dylib"
    } else if cfg!(target_os = "windows") {
        "/release/resources/native/pdfium/bin/pdfium.dll"
    } else {
        "/release/resources/native/pdfium/lib/libpdfium.so"
    };
    assert_eq!(runtime.library_path(), PathBuf::from(expected));
}

#[test]
fn missing_runtime_library_is_reported_before_binding() {
    let runtime = PdfiumRuntime::from_bundled_root("/definitely-missing-zeta-pdfium");
    assert!(matches!(
        PdfTextExtractor::bind(runtime),
        Err(DocumentError::PdfiumLibraryMissing { .. })
    ));
}

#[test]
fn extracts_text_with_a_release_staged_runtime_when_configured() {
    let Some(runtime_root) = env::var_os("ZETA_PDFIUM_ROOT") else {
        return;
    };
    let Some(document_path) = env::var_os("ZETA_PDFIUM_TEST_DOCUMENT") else {
        return;
    };

    let extractor = PdfTextExtractor::bind(PdfiumRuntime::from_bundled_root(runtime_root))
        .expect("bind release-staged PDFium");
    let extracted = extractor
        .extract_file(document_path)
        .expect("extract native PDF text");

    assert!(extracted.page_count() > 0);
    assert_eq!(extracted.pages[0].page_number, 1);
    assert!(
        extracted
            .pages
            .iter()
            .any(|page| !page.text.trim().is_empty()),
        "extracted text was {:?}",
        extracted.pages
    );
}
