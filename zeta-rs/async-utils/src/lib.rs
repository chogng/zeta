//! Runtime-independent cooperative cancellation for asynchronous work.
//!
//! A [`CancellationSource`] owns permission to request cancellation, while a
//! [`CancellationToken`] lets code observe cancellation and create isolated child
//! cancellation domains. Use [`FutureCancellationExt::with_cancellation`] when it is safe to
//! drop a future as soon as cancellation wins. Futures that require graceful cleanup should
//! instead await [`CancellationToken::cancelled`] or call [`CancellationToken::check`].

mod future;
mod source;
mod tree;

pub use future::{Cancelable, FutureCancellationExt};
pub use source::{
    CancelOnDrop, CancelResult, Cancellation, CancellationId, CancellationReason,
    CancellationSource, CancellationToken, Cancelled,
};
