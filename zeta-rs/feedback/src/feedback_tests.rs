use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct Client(Mutex<Vec<zeta_client::ClientRequest>>);
impl zeta_client::OperationClient for Client {
    fn execute(
        &self,
        request: &zeta_client::ClientRequest,
    ) -> Result<zeta_client::ClientResponse, zeta_client::ClientError> {
        self.0.lock().unwrap().push(request.clone());
        Ok(zeta_http_client::HttpResponse::new(204, vec![], vec![]))
    }
}

fn snapshot() -> DiagnosticSnapshot {
    diagnostics::Diagnostics::default().snapshot(Default::default())
}

#[test]
fn upload_requires_the_reviewed_digest_and_owner_and_is_consumed_once() {
    let feedback = Feedback::default();
    let prepared = feedback
        .prepare(1, "https://feedback.example.test/submit", snapshot())
        .unwrap();
    let client = Client::default();
    let token = zeta_async_utils::CancellationSource::new().token();
    assert!(
        feedback
            .upload(2, &prepared.digest, &client, &token)
            .is_err()
    );
    assert!(feedback.upload(1, "wrong digest", &client, &token).is_err());
    assert!(client.0.lock().unwrap().is_empty());
    feedback
        .upload(1, &prepared.digest, &client, &token)
        .unwrap();
    assert!(
        feedback
            .upload(1, &prepared.digest, &client, &token)
            .is_err()
    );
    let calls = client.0.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].body(), prepared.content.as_bytes());
    assert_eq!(calls[0].url(), prepared.endpoint);
    assert_eq!(calls[0].retry_policy().max_attempts().get(), 1);
}

#[test]
fn destination_and_connection_close_invalidate_consent() {
    let feedback = Feedback::default();
    for endpoint in [
        "http://feedback.example.test",
        "https://user:secret@feedback.example.test",
        "https://feedback.example.test/?token=secret",
    ] {
        assert!(feedback.prepare(1, endpoint, snapshot()).is_err());
    }
    let one = feedback
        .prepare(1, "https://one.example.test", snapshot())
        .unwrap();
    let two = feedback
        .prepare(1, "https://two.example.test", snapshot())
        .unwrap();
    assert_ne!(one.digest, two.digest);
    feedback.close(1);
    assert!(
        feedback
            .upload(
                1,
                &one.digest,
                &Client::default(),
                &zeta_async_utils::CancellationSource::new().token()
            )
            .is_err()
    );
}
