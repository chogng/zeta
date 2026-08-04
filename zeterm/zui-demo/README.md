# `zui-demo`

`zui-demo` is a framework-only host smoke test. It composes a `zeta-ui::ActionBar` with a local
`zui::Icon`, submits the resulting `UiScene` through the `zeta-renderer::Renderer` contract, and
uses an in-memory recording backend. It intentionally has no dependency on `zeterm`, commands,
terminal sessions, App Server state, `zeta-icons`, `winit`, or `wgpu`.

The crate is not a production application. Its purpose is to keep the portability claim
executable: if a reusable component starts requiring product state or a platform backend, this
host should fail to remain small and dependency-neutral.

Verification:

```text
cargo test -p zui-demo
cargo run -p zui-demo
```
