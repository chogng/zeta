use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Legacy durable binding from a local Thread to an external full-Turn execution runtime.
///
/// Zeta no longer creates these bindings. The type remains serializable so existing Thread history
/// can still be read without discarding its original recovery facts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnExecutionBinding {
    pub backend: String,
    pub remote_thread_id: String,
    /// Opaque authority scope in which the external thread was created.
    ///
    /// Adapters must compare this value before resuming the remote thread. It intentionally
    /// excludes filesystem paths and credentials while preventing cross-workspace reuse.
    pub execution_scope: String,
}
