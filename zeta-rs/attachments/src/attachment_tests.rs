use std::io::Cursor;
use std::sync::Arc;
use std::sync::Mutex;

use image::DynamicImage;
use image::ImageFormat;
use zeta_http_client::HttpClient;
use zeta_http_client::HttpClientError;
use zeta_http_client::HttpHeader;
use zeta_http_client::HttpRequest;
use zeta_http_client::HttpResponse;
use zeta_protocol::ImageDetail;

use crate::FileImageAttachmentStore;
use crate::ImageAttachments;
use crate::SafeRemoteImageFetcher;

#[test]
fn file_store_round_trips_across_service_instances() {
    let root = tempfile::tempdir().unwrap();
    let first = ImageAttachments::new(Arc::new(
        FileImageAttachmentStore::open(root.path()).unwrap(),
    ));
    let reference = first
        .import_bytes(test_png(4, 3), ImageDetail::Auto)
        .unwrap();

    let reopened = ImageAttachments::new(Arc::new(
        FileImageAttachmentStore::open(root.path()).unwrap(),
    ));
    let data_url = reopened.materialize_data_url(&reference).unwrap();

    assert!(data_url.starts_with("data:image/png;base64,"));
    assert_eq!(reference.width, 4);
    assert_eq!(reference.height, 3);
}

#[test]
fn duplicate_content_reuses_the_same_reference() {
    let service = ImageAttachments::in_memory();
    let bytes = test_png(2, 2);

    let first = service
        .import_bytes(bytes.clone(), ImageDetail::Auto)
        .unwrap();
    let second = service.import_bytes(bytes, ImageDetail::Auto).unwrap();

    assert_eq!(first, second);
}

#[test]
fn forged_reference_metadata_is_rejected_on_read() {
    let service = ImageAttachments::in_memory();
    let mut reference = service
        .import_bytes(test_png(2, 2), ImageDetail::Auto)
        .unwrap();
    reference.width = 7;

    assert!(service.materialize_data_url(&reference).is_err());
}

#[test]
fn corrupted_file_store_objects_are_rejected_on_read() {
    let root = tempfile::tempdir().unwrap();
    let service = ImageAttachments::new(Arc::new(
        FileImageAttachmentStore::open(root.path()).unwrap(),
    ));
    let reference = service
        .import_bytes(test_png(2, 2), ImageDetail::Auto)
        .unwrap();
    let hex = reference
        .content_digest
        .as_str()
        .strip_prefix("sha256:")
        .unwrap();
    let object = root.path().join("sha256").join(&hex[..2]).join(hex);
    std::fs::write(object, b"corrupt").unwrap();

    assert!(service.materialize_data_url(&reference).is_err());
}

#[test]
fn remote_redirects_are_revalidated_without_forwarding_credentials() {
    let client = Arc::new(ScriptedHttpClient::new(vec![
        HttpResponse::new(
            302,
            vec![HttpHeader::new(
                "location",
                "https://cdn.example.test/image.png",
            )],
            Vec::new(),
        ),
        HttpResponse::new(200, Vec::new(), test_png(1, 1)),
    ]));
    let fetcher = SafeRemoteImageFetcher::with_client(client.clone());

    let bytes = crate::RemoteImageFetcher::fetch(&fetcher, "https://example.test/start").unwrap();

    assert!(!bytes.is_empty());
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1], "https://cdn.example.test/image.png");
}

#[test]
fn remote_https_redirect_cannot_downgrade_or_include_credentials() {
    let client = Arc::new(ScriptedHttpClient::new(vec![HttpResponse::new(
        302,
        vec![HttpHeader::new("location", "http://example.test/image.png")],
        Vec::new(),
    )]));
    let fetcher = SafeRemoteImageFetcher::with_client(client.clone());

    assert!(crate::RemoteImageFetcher::fetch(&fetcher, "https://example.test/start").is_err());
    assert!(
        crate::RemoteImageFetcher::fetch(&fetcher, "https://user:secret@example.test/image.png")
            .is_err()
    );
    assert_eq!(client.requests.lock().unwrap().len(), 1);
}

#[test]
fn remote_urls_reject_nonstandard_and_cross_scheme_ports() {
    let client = Arc::new(ScriptedHttpClient::new(Vec::new()));
    let fetcher = SafeRemoteImageFetcher::with_client(client.clone());

    for url in [
        "http://example.test:443/image.png",
        "https://example.test:80/image.png",
        "https://example.test:8443/image.png",
    ] {
        assert!(crate::RemoteImageFetcher::fetch(&fetcher, url).is_err());
    }
    assert!(client.requests.lock().unwrap().is_empty());
}

fn test_png(width: u32, height: u32) -> Vec<u8> {
    let image = DynamicImage::new_rgba8(width, height);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}

struct ScriptedHttpClient {
    responses: Mutex<Vec<HttpResponse>>,
    requests: Mutex<Vec<String>>,
}

impl ScriptedHttpClient {
    fn new(mut responses: Vec<HttpResponse>) -> Self {
        responses.reverse();
        Self {
            responses: Mutex::new(responses),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl HttpClient for ScriptedHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError> {
        self.requests.lock().unwrap().push(request.url().to_owned());
        self.responses
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| HttpClientError::Transport("script exhausted".into()))
    }
}
