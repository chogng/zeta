//! app's adapter for the product-neutral Remote Tunnel host.
//!
//! The lifecycle supervisor lives in `zeta-remote-host`. This module only translates its typed
//! events into app's `ProductEvent` channel and keeps the host owned by the desktop application.

use std::num::NonZeroU16;
use std::path::PathBuf;

use zeta_remote::SshHost;
use zeta_remote_host::RemoteTunnelHost as SharedRemoteTunnelHost;
use zui::app::AppProxy;

use crate::product_event::ProductEvent;

pub(crate) use zeta_remote_host::RemoteTunnelEvent;
pub(crate) use zeta_remote_host::RemoteTunnelId;

/// app-owned adapter around the shared Remote Tunnel supervisor.
pub(crate) struct ProductRemoteTunnelHost {
    inner: SharedRemoteTunnelHost,
}

impl ProductRemoteTunnelHost {
    pub(crate) fn new(host: SshHost, ssh_executable: impl Into<PathBuf>) -> Self {
        Self {
            inner: SharedRemoteTunnelHost::new(host, ssh_executable),
        }
    }

    pub(crate) fn host(&self) -> &SshHost {
        self.inner.host()
    }

    pub(crate) fn start(
        &mut self,
        remote_port: NonZeroU16,
        event_proxy: AppProxy<ProductEvent>,
    ) -> Result<RemoteTunnelId, String> {
        self.inner.start(remote_port, move |event| {
            let _ = event_proxy.send_event(ProductEvent::RemoteTunnel(event));
        })
    }

    pub(crate) fn stop(&self, tunnel_id: RemoteTunnelId) -> bool {
        self.inner.stop(tunnel_id)
    }

    pub(crate) fn handle_event(&mut self, event: &RemoteTunnelEvent) -> bool {
        self.inner.handle_event(event)
    }
}
