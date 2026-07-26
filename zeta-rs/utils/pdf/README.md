# `zeta-pdf`

`zeta-pdf` is the native document-ingestion boundary. Its first capability
is PDFium-backed, page-by-page text extraction with one-based page numbers for
citations and viewer jumps.

It does not own OCR, chunks, embeddings, vector retrieval, document metadata,
or agent memory. Those consumers receive `ExtractedPdfDocument` and keep their
own persistence and retrieval policies.

## Runtime contract

The release build must first stage PDFium into its resources directory:

```sh
node scripts/fetch-pdfium.mjs --target darwin-arm64 \
  --output "$STAGING/resources/native/pdfium"
```

At app-server composition, bind exactly that root:

```rust
let runtime = PdfiumRuntime::from_bundled_root(
    resources_root.join("native/pdfium"),
);
let extractor = PdfTextExtractor::bind(runtime)?;
```

No system-installed PDFium and no ambient dynamic-library search path is used.

## Native smoke test

The ordinary unit tests do not require a native library. To additionally bind
the release-staged library and extract a known PDF, set both variables:

```sh
ZETA_PDFIUM_ROOT="$STAGING/resources/native/pdfium" \
ZETA_PDFIUM_TEST_DOCUMENT=/absolute/path/to/fixture.pdf \
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-pdf
```
