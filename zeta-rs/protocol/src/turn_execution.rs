use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Durable binding from a local Thread to an external full-Turn execution runtime.
///
/// The remote identity is opaque to Core. Adapters may use it only to resume a completed
/// conversation; an in-flight Turn still has unknown outcome after process loss and is never
/// replayed from this binding.
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
