//! Stable command identities and runtime-free registration primitives for `zeterm`.

mod command;
mod registry;
mod request;

pub use command::ZetermCommandId;
pub use registry::CommandHandler;
pub use registry::CommandRegistry;
pub use registry::CommandRegistryError;
pub use request::CommandRequest;
