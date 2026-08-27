//! Stable command identities and runtime-free registration primitives for `app`.

mod command;
mod registry;
mod request;

pub use command::AppCommandId;
pub use registry::CommandHandler;
pub use registry::CommandRegistry;
pub use registry::CommandRegistryError;
pub use request::CommandRequest;
