use std::fmt;
use std::sync::Arc;

use futures::stream;
use tough::Bytes;
use tough::Transport;
use tough::TransportError;
use tough::TransportErrorKind;
use tough::TransportStream;
use url::Url;
use zeta_http_client::HttpClient;
use zeta_http_client::HttpHeader;
use zeta_http_client::HttpMethod;
use zeta_http_client::HttpRequest;

#[derive(Clone)]
pub(crate) struct MarketplaceTransport {
    http: Arc<dyn HttpClient>,
}

impl MarketplaceTransport {
    pub(crate) fn new(http: Arc<dyn HttpClient>) -> Self {
        Self { http }
    }
}

impl fmt::Debug for MarketplaceTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MarketplaceTransport")
            .finish_non_exhaustive()
    }
}

#[tough::async_trait]
impl Transport for MarketplaceTransport {
    async fn fetch(&self, url: Url) -> Result<TransportStream, TransportError> {
        if url.scheme() != "https" {
            return Err(TransportError::new(
                TransportErrorKind::UnsupportedUrlScheme,
                url.as_str(),
            ));
        }
        let request = HttpRequest::new(
            HttpMethod::Get,
            url.as_str(),
            vec![HttpHeader::new("Accept", "application/octet-stream")],
            Vec::new(),
        )
        .map_err(|error| {
            TransportError::new_with_cause(TransportErrorKind::Other, url.as_str(), error)
        })?;
        let response = self.http.execute(&request).map_err(|error| {
            TransportError::new_with_cause(TransportErrorKind::Other, url.as_str(), error)
        })?;
        if response.status() == 404 {
            return Err(TransportError::new(
                TransportErrorKind::FileNotFound,
                url.as_str(),
            ));
        }
        if !response.is_success() {
            return Err(TransportError::new(TransportErrorKind::Other, url.as_str()));
        }
        let body = Bytes::copy_from_slice(response.body());
        Ok(Box::pin(stream::once(async move { Ok(body) })))
    }
}
