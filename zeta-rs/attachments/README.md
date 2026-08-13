# zeta-attachments

`zeta-attachments` owns admission and durable identity for image bytes used by Threads. It sits
between untrusted local/remote/upload inputs and `zeta-core`; model providers never read this
store directly.

The canonical path is:

```text
local bytes / remote URL / Tool image
  -> bounded fetch or upload
  -> zeta-utils-image validation and normalization
  -> ImageAttachmentStore put-before-event
  -> ImageAttachmentRef in Thread history
  -> verified read and ephemeral data URL at model invocation
```

`FileImageAttachmentStore` writes content-addressed objects below `attachments/sha256`, fsyncs a
same-directory temporary file, and publishes it without replacing an existing object. A failed
Thread append can leave a harmless unreferenced object; garbage collection is intentionally a
separate maintenance operation.

`ImageAttachments` is the canonical admission/read service. `reference_for_image` derives trusted
metadata only after `zeta-utils-image` validation; `verify_reference_bytes` rechecks digest, byte
length, encoding signature and dimensions whenever a reference is read. Product hosts install a
`FileImageAttachmentStore`; tests and explicitly ephemeral hosts may use
`MemoryImageAttachmentStore`.

`SafeRemoteImageFetcher` uses a direct, redirect-rejecting HTTP client whose actual resolver
rejects loopback, private, link-local, multicast, documentation, benchmark, and unspecified
addresses. Redirects are followed manually and revalidated; HTTPS cannot downgrade to HTTP.

Current limitation: this crate does not yet implement reference tracing or orphan garbage
collection. Removing an object before every durable Thread reference has expired is invalid.
