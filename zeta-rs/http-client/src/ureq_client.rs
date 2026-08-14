use crate::ClientIdentityPolicy;
use crate::HttpClientConfig;
use crate::HttpClientError;
use crate::HttpRequest;
use crate::HttpResponse;
use crate::NetworkTargetPolicy;
use crate::ProxyBypass;
use crate::ProxyPolicy;
use crate::RedirectPolicy;
use crate::Timeout;
use crate::TlsPolicy;
use std::io;
use std::io::Read;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::OnceLock;

type SystemRootLoader =
    Arc<dyn Fn() -> Result<rustls::RootCertStore, HttpClientError> + Send + Sync>;

/// Executes a fully constructed HTTP request once.
///
/// Implementations own proxy selection, TLS server validation, redirect
/// handling, transport timeouts, and connection reuse. They must not retry:
/// the operation client above this trait decides whether a request body is safe
/// to replay.
pub trait HttpClient: Send + Sync {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError>;

    /// Executes one request and incrementally emits a successful response body.
    ///
    /// Non-success response bodies remain buffered in the returned response so
    /// operation-level retry and status handling can inspect them. The default
    /// bridge preserves compatibility for transports that only implement unary
    /// execution.
    fn execute_streaming(
        &self,
        request: &HttpRequest,
        sink: &mut dyn HttpBodySink,
    ) -> Result<HttpResponse, HttpClientError> {
        let response = self.execute(request)?;
        if !response.is_success() {
            return Ok(response);
        }
        sink.emit(response.body())?;
        Ok(HttpResponse::new(
            response.status(),
            response.headers().to_vec(),
            Vec::new(),
        ))
    }
}

/// Receives ordered byte chunks from one successful HTTP response body.
///
/// Implementations should apply backpressure and return an error as soon as
/// the consumer can no longer safely accept bytes. A sink is scoped to one raw
/// transport attempt and must not interpret provider framing.
pub trait HttpBodySink {
    fn emit(&mut self, chunk: &[u8]) -> Result<(), HttpClientError>;
}

/// The production synchronous HTTP client backed by one reusable `ureq` agent.
pub struct UreqHttpClient {
    config: HttpClientConfig,
    http_direct_agent: ureq::Agent,
    http_proxy_agent: Option<ureq::Agent>,
    proxy_url: Option<String>,
    proxy_bypass: ProxyBypass,
    response_body_limit: usize,
    streaming_response_body_limit: usize,
    system_root_loader: SystemRootLoader,
    secure_tls_config: OnceLock<Result<Arc<rustls::ClientConfig>, HttpClientError>>,
    https_direct_agent: OnceLock<Result<ureq::Agent, HttpClientError>>,
    https_proxy_agent: OnceLock<Result<ureq::Agent, HttpClientError>>,
}

impl UreqHttpClient {
    /// Builds the production client with the default transport policy.
    ///
    /// Static proxy, custom-certificate, and client-identity configuration is validated here.
    /// Platform certificate roots are loaded lazily on the first HTTPS request so an HTTP-only
    /// client remains usable in offline or restricted hosts.
    pub fn new() -> Result<Self, HttpClientError> {
        Self::with_config(HttpClientConfig::default())
    }

    pub fn with_config(config: HttpClientConfig) -> Result<Self, HttpClientError> {
        Self::with_config_and_root_loader(config, Arc::new(system_root_store))
    }

    fn with_config_and_root_loader(
        config: HttpClientConfig,
        system_root_loader: SystemRootLoader,
    ) -> Result<Self, HttpClientError> {
        if config.network_targets() == NetworkTargetPolicy::PublicInternetOnly
            && (!matches!(config.proxy(), ProxyPolicy::Direct)
                || config.redirects() != RedirectPolicy::Reject)
        {
            return Err(HttpClientError::InvalidConfiguration(
                "public-Internet target filtering requires direct connections and rejected redirects"
                    .into(),
            ));
        }
        let http_tls_config = build_tls_config(&config, SystemRoots::Skip, &system_root_loader)?;
        let http_direct_agent = build_agent(&config, http_tls_config.clone(), None)?;
        let (proxy_url, proxy_bypass) = resolve_proxy(config.proxy());
        let http_proxy_agent = proxy_url
            .as_deref()
            .map(|proxy_url| build_agent(&config, http_tls_config, Some(proxy_url)))
            .transpose()?;
        let response_body_limit = config.response_body_limit().bytes().get();
        let streaming_response_body_limit = config.streaming_response_body_limit().bytes().get();

        Ok(Self {
            config,
            http_direct_agent,
            http_proxy_agent,
            proxy_url,
            proxy_bypass,
            response_body_limit,
            streaming_response_body_limit,
            system_root_loader,
            secure_tls_config: OnceLock::new(),
            https_direct_agent: OnceLock::new(),
            https_proxy_agent: OnceLock::new(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_test_system_root_loader(
        config: HttpClientConfig,
        loader: impl Fn() -> Result<rustls::RootCertStore, HttpClientError> + Send + Sync + 'static,
    ) -> Result<Self, HttpClientError> {
        Self::with_config_and_root_loader(config, Arc::new(loader))
    }

    fn agent_for(&self, request: &HttpRequest) -> Result<&ureq::Agent, HttpClientError> {
        let use_proxy = self.uses_proxy(request);
        let proxy_requires_tls = use_proxy
            && self
                .proxy_url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://"));
        let request_requires_tls = request.url().starts_with("https://")
            || matches!(self.config.redirects(), RedirectPolicy::Follow { .. });
        if !request_requires_tls && !proxy_requires_tls {
            return if use_proxy {
                Ok(self
                    .http_proxy_agent
                    .as_ref()
                    .expect("a selected proxy always has an agent"))
            } else {
                Ok(&self.http_direct_agent)
            };
        }

        if use_proxy {
            self.https_proxy_agent
                .get_or_init(|| {
                    build_agent(
                        &self.config,
                        self.secure_tls_config()?,
                        self.proxy_url.as_deref(),
                    )
                })
                .as_ref()
                .map_err(Clone::clone)
        } else {
            self.https_direct_agent
                .get_or_init(|| build_agent(&self.config, self.secure_tls_config()?, None))
                .as_ref()
                .map_err(Clone::clone)
        }
    }

    fn uses_proxy(&self, request: &HttpRequest) -> bool {
        if self.proxy_url.is_none() {
            return false;
        };
        let Some((host, port)) = request_authority(request.url()) else {
            return true;
        };
        !self.proxy_bypass.matches(host, port)
    }

    fn secure_tls_config(&self) -> Result<Arc<rustls::ClientConfig>, HttpClientError> {
        self.secure_tls_config
            .get_or_init(|| {
                build_tls_config(&self.config, SystemRoots::Load, &self.system_root_loader)
            })
            .as_ref()
            .cloned()
            .map_err(Clone::clone)
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
    if config.network_targets() == NetworkTargetPolicy::PublicInternetOnly {
        builder = builder.resolver(resolve_public_internet_target);
    }
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

fn resolve_public_internet_target(netloc: &str) -> io::Result<Vec<SocketAddr>> {
    let addresses = netloc.to_socket_addrs()?.collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "target resolved to no addresses",
        ));
    }
    if addresses
        .iter()
        .any(|address| !is_public_internet_ip(address.ip()))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "target resolved to a non-public address",
        ));
    }
    Ok(addresses)
}

pub(crate) fn is_public_internet_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    !matches!(
        (a, b, c),
        (0, _, _)
            | (10, _, _)
            | (100, 64..=127, _)
            | (127, _, _)
            | (169, 254, _)
            | (172, 16..=31, _)
            | (192, 0, 0..=2)
            | (192, 88, 99)
            | (192, 168, _)
            | (198, 18..=19, _)
            | (198, 51, 100)
            | (203, 0, 113)
            | (224..=255, _, _)
    )
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(address) = address.to_ipv4_mapped() {
        return is_public_ipv4(address);
    }
    let segments = address.segments();
    !address.is_unspecified()
        && !address.is_loopback()
        && !address.is_multicast()
        && !segments[..6].iter().all(|segment| *segment == 0)
        && !(segments[0] == 0x0064 && segments[1] == 0xff9b)
        && !(segments[0] == 0x0100 && segments[1..4].iter().all(|segment| *segment == 0))
        && !(segments[0] == 0x2001 && segments[1] <= 0x01ff)
        && segments[0] != 0x2002
        && segments[0] & 0xfff0 != 0x3ff0
        && segments[0] != 0x5f00
        && segments[0] & 0xfe00 != 0xfc00
        && segments[0] & 0xffc0 != 0xfe80
        && segments[0] & 0xffc0 != 0xfec0
        && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
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
    system_roots: SystemRoots,
    system_root_loader: &SystemRootLoader,
) -> Result<Arc<rustls::ClientConfig>, HttpClientError> {
    let mut roots = match config.tls() {
        TlsPolicy::SystemRoots | TlsPolicy::SystemPlus(_) => match system_roots {
            SystemRoots::Load => system_root_loader()?,
            SystemRoots::Skip => rustls::RootCertStore::empty(),
        },
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

#[derive(Clone, Copy)]
enum SystemRoots {
    Load,
    Skip,
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
        let response = self.send(request)?;
        let status = response.status();
        let headers = response_headers(&response);
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

    fn execute_streaming(
        &self,
        request: &HttpRequest,
        sink: &mut dyn HttpBodySink,
    ) -> Result<HttpResponse, HttpClientError> {
        let response = self.send(request)?;
        let status = response.status();
        let headers = response_headers(&response);
        if !(200..300).contains(&status) {
            let mut body = Vec::new();
            read_bounded(response.into_reader(), self.response_body_limit, |chunk| {
                body.extend_from_slice(chunk);
                Ok(())
            })?;
            return Ok(HttpResponse::new(status, headers, body));
        }
        read_bounded(
            response.into_reader(),
            self.streaming_response_body_limit,
            |chunk| sink.emit(chunk),
        )?;
        Ok(HttpResponse::new(status, headers, Vec::new()))
    }
}

impl UreqHttpClient {
    fn send(&self, request: &HttpRequest) -> Result<ureq::Response, HttpClientError> {
        let mut request_builder = self
            .agent_for(request)?
            .request(request.method().as_str(), request.url());
        for header in request.headers() {
            request_builder = request_builder.set(header.name(), header.value());
        }
        match request_builder.send_bytes(request.body()) {
            Ok(response) | Err(ureq::Error::Status(_, response)) => Ok(response),
            Err(ureq::Error::Transport(_)) => {
                Err(HttpClientError::Transport("request failed".into()))
            }
        }
    }
}

fn response_headers(response: &ureq::Response) -> Vec<crate::HttpHeader> {
    response
        .headers_names()
        .iter()
        .filter_map(|name| {
            response
                .header(name)
                .map(|value| crate::HttpHeader::new(name.as_str(), value))
        })
        .collect()
}

fn read_bounded(
    mut reader: impl Read,
    limit: usize,
    mut emit: impl FnMut(&[u8]) -> Result<(), HttpClientError>,
) -> Result<(), HttpClientError> {
    let mut total = 0usize;
    let mut chunk = [0u8; 8 * 1024];
    loop {
        let read = reader
            .read(&mut chunk)
            .map_err(|_| HttpClientError::Transport("failed to read response body".into()))?;
        if read == 0 {
            return Ok(());
        }
        total = total.saturating_add(read);
        if total > limit {
            return Err(HttpClientError::Transport(
                "response body exceeded configured limit".into(),
            ));
        }
        emit(&chunk[..read])?;
    }
}
