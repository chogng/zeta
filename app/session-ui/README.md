# zeta-session-ui

Session feature state and UI-facing behavior.

The first extracted capability is Thread snapshot and incremental-update merging. The app host
continues to own App Server transport, window routing, and side effects; this crate only exposes
typed state and result values.
