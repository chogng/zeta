use tempfile::TempDir;
use url::Url;

use super::MarketplaceRemoteClient;
use crate::RemoteMarketplaceConfig;

#[test]
fn opening_the_remote_client_never_requires_network_access() {
    let cache = TempDir::new().unwrap();
    let config = RemoteMarketplaceConfig::new(
        Url::parse("https://unreachable.invalid/metadata/").unwrap(),
        Url::parse("https://unreachable.invalid/targets/").unwrap(),
        b"not-yet-parsed-because-access-is-lazy".to_vec(),
        cache.path(),
    )
    .unwrap();

    MarketplaceRemoteClient::open(config).unwrap();
}
