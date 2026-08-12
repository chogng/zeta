use crate::{
    ClientIdentityPolicy, HttpClientConfig, HttpClientError, HttpRequest, HttpResponse,
    ProxyBypass, ProxyPolicy, RedirectPolicy, Timeout, TlsPolicy,
};
use std::io::Read;
use std::sync::Arc;

/// Executes a fully constructed HTTP request once.
///
/// Implementations own proxy selection, TLS server validation, redirect
/// handling, transport timeouts, and connection reuse. They must not retry:
/// the operation client above this trait decides whether a request body is safe
/// to replay.
pub trait HttpClient: Send + Sync {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError>;
}

/// The production synchronous HTTP client backed by one reusable `ureq` agent.
pub struct UreqHttpClient {
    direct_agent: ureq::Agent,
    proxy_agent: Option<ureq::Agent>,
    proxy_bypass: ProxyBypass,
    response_body_limit: usize,
}

impl UreqHttpClient {
    /// Builds the production client with the default transport policy.
    ///
    /// Loading platform roots and proxy configuration can fail in restricted hosts, so callers
    /// must handle the result instead of assuming that process startup implies network access.
    pub fn new() -> Result<Self, HttpClientError> {
        Self::with_config(HttpClientConfig::default())
    }

    pub fn with_config(config: HttpClientConfig) -> Result<Self, HttpClientError> {
        let tls_config = build_tls_config(&config)?;
        let direct_agent = build_agent(&config, tls_config.clone(), None)?;
        let (proxy_url, proxy_bypass) = resolve_proxy(config.proxy());
        let proxy_agent = proxy_url
            .as_deref()
            .map(|proxy_url| build_agent(&config, tls_config, Some(proxy_url)))
            .transpose()?;

        Ok(Self {
            direct_agent,
            proxy_agent,
            proxy_bypass,
            response_body_limit: config.response_body_limit().bytes().get(),
        })
    }

    fn agent_for(&self, request: &HttpRequest) -> &ureq::Agent {
        let Some(proxy_agent) = self.proxy_agent.as_ref() else {
            return &self.direct_agent;
        };
        let Some((host, port)) = request_authority(request.url()) else {
            return proxy_agent;
        };
        if self.proxy_bypass.matches(host, port) {
            &self.direct_agent
        } else {
            proxy_agent
        }
    }
}

fn resolve_proxy(policy: &ProxyPolicy) -> (Option<String>, ProxyBypass) {
    match policy {
        ProxyPolicy::Direct => (None, ProxyBypass::from_comma_separated("")),
        ProxyPolicy::FromEnvironment => (
            proxy_url_from_environment(),
            ProxyBypass::from_environment(),
        ),
        ProxyPolicy::Explicit(proxy_url) => (
            Some(proxy_url.clone()),
            ProxyBypass::from_comma_separated(""),
        ),
        ProxyPolicy::ExplicitWithBypass { proxy_url, bypass } => {
            (Some(proxy_url.clone()), bypass.clone())
        }
    }
}

fn proxy_url_from_environment() -> Option<String> {
    [
        "ALL_PROXY",
        "all_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
}

fn build_agent(
    config: &HttpClientConfig,
    tls_config: Arc<rustls::ClientConfig>,
    proxy_url: Option<&str>,
) -> Result<ureq::Agent, HttpClientError> {
    let mut builder = ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .max_idle_connections(config.connection_pool().max_idle_connections())
        .max_idle_connections_per_host(config.connection_pool().max_idle_connections_per_host())
        .tls_config(tls_config);
    if let Some(proxy_url) = proxy_url {
        let proxy = ureq::Proxy::new(proxy_url)
            .map_err(|_| HttpClientError::InvalidConfiguration("proxy URL is invalid".into()))?;
        builder = builder.proxy(proxy);
    }
    builder = match config.redirects() {
        RedirectPolicy::Reject => builder.redirects(0),
        RedirectPolicy::Follow { max_hops } => builder.redirects(u32::from(max_hops.get())),
    };
    let timeouts = config.timeouts();
    if let Timeout::After(timeout) = timeouts.connect() {
        builder = builder.timeout_connect(timeout);
    }
    if let Timeout::After(timeout) = timeouts.read() {
        builder = builder.timeout_read(timeout);
    }
    if let Timeout::After(timeout) = timeouts.write() {
        builder = builder.timeout_write(timeout);
    }
    if let Timeout::After(timeout) = timeouts.overall() {
        builder = builder.timeout(timeout);
    }
    Ok(builder.build())
}

fn request_authority(url: &str) -> Option<(&str, Option<u16>)> {
    let authority_and_path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()?
        .rsplit('@')
        .next()?;
    if authority.is_empty() {
        return None;
    }
    if let Some(bracket_end) = authority.find(']')
        && authority.starts_with('[')
    {
        let host = &authority[1..bracket_end];
        let port = authority[bracket_end + 1..]
            .strip_prefix(':')
            .and_then(|port| port.parse().ok());
        return Some((host, port));
    }
    let Some((host, port)) = authority.rsplit_once(':') else {
        return Some((authority, None));
    };
    if host.contains(':') {
        Some((authority, None))
    } else {
        Some((host, port.parse().ok()))
    }
}

fn build_tls_config(
    config: &HttpClientConfig,
) -> Result<Arc<rustls::ClientConfig>, HttpClientError> {
    let mut roots = match config.tls() {
        TlsPolicy::SystemRoots | TlsPolicy::SystemPlus(_) => system_root_store()?,
        TlsPolicy::CustomOnly(_) => rustls::RootCertStore::empty(),
    };
    match config.tls() {
        TlsPolicy::SystemRoots => {}
        TlsPolicy::SystemPlus(bundle) | TlsPolicy::CustomOnly(bundle) => {
            add_certificate_bundle(&mut roots, bundle.certificates())?;
        }
    }

    let builder = rustls::ClientConfig::builder_with_provider(
        rustls::crypto::ring::default_provider().into(),
    )
    .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
    .map_err(|_| {
        HttpClientError::InvalidConfiguration("TLS provider has no usable protocol versions".into())
    })?
    .with_root_certificates(roots);
    let tls_config = match config.client_identity() {
        ClientIdentityPolicy::None => builder.with_no_client_auth(),
        ClientIdentityPolicy::Identity(identity) => builder
            .with_client_auth_cert(
                identity.certificate_chain().collect(),
                identity.private_key()?,
            )
            .map_err(|_| {
                HttpClientError::InvalidConfiguration(
                    "client certificate chain and private key do not match".into(),
                )
            })?,
    };
    Ok(Arc::new(tls_config))
}

fn system_root_store() -> Result<rustls::RootCertStore, HttpClientError> {
    let native_certificates = rustls_native_certs::load_native_certs().map_err(|_| {
        HttpClientError::InvalidConfiguration("failed to load system certificate roots".into())
    })?;
    let mut roots = rustls::RootCertStore::empty();
    let (valid_count, _) = roots.add_parsable_certificates(native_certificates);
    if valid_count == 0 {
        return Err(HttpClientError::InvalidConfiguration(
            "system certificate roots are empty".into(),
        ));
    }
    Ok(roots)
}

fn add_certificate_bundle(
    roots: &mut rustls::RootCertStore,
    certificates: impl IntoIterator<Item = rustls::pki_types::CertificateDer<'static>>,
) -> Result<(), HttpClientError> {
    for certificate in certificates {
        roots.add(certificate).map_err(|_| {
            HttpClientError::InvalidConfiguration(
                "certificate bundle contains an invalid trust root".into(),
            )
        })?;
    }
    Ok(())
}

impl HttpClient for UreqHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError> {
        let mut request_builder = self
            .agent_for(request)
            .request(request.method().as_str(), request.url());
        for header in request.headers() {
            request_builder = request_builder.set(header.name(), header.value());
        }
        let response = match request_builder.send_bytes(request.body()) {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(_)) => {
                return Err(HttpClientError::Transport("request failed".into()));
            }
        };
        let status = response.status();
        let headers = response
            .headers_names()
            .iter()
            .filter_map(|name| {
                response
                    .header(name)
                    .map(|value| crate::HttpHeader::new(name.as_str(), value))
            })
            .collect();
        let mut body = Vec::new();
        let read_limit = u64::try_from(self.response_body_limit)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        response
            .into_reader()
            .take(read_limit)
            .read_to_end(&mut body)
            .map_err(|_| HttpClientError::Transport("failed to read response body".into()))?;
        if body.len() > self.response_body_limit {
            return Err(HttpClientError::Transport(
                "response body exceeded configured limit".into(),
            ));
        }
        Ok(HttpResponse::new(status, headers, body))
    }
}
