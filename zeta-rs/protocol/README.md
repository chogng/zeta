# zeta-protocol

`zeta-protocol` is Zeta's provider-independent shared semantic contract. It contains canonical
models, intents, durable facts, consumer updates, identities, and provider-neutral values; it
contains no runtime, transport, storage implementation, provider wire format, or effect policy.

The complete ownership rules, current implementation audit, known gaps, and phased evolution plan
live in [`docs/protocol.md`](../../docs/protocol.md). That document is the source of truth for this
crate's architecture; this README is only its package landing page.
