# `zui-demo`

`zui-demo` is a framework-only host smoke test. It composes a `zeta-ui::ActionBar` with a local
`zui::ui::Icon`, submits the resulting `UiScene` through the `zui::render::Renderer` contract, and
uses an in-memory recording backend. It intentionally has no dependency on `zeterm`, commands,
terminal sessions, App Server state, `zeta-icons`, `winit`, or `wgpu`.

The crate is not a production application. Its purpose is to keep the portability claim
executable: if a reusable component starts requiring product state or a platform backend, this
host should fail to remain small and dependency-neutral.

The optional `zui-native-demo` binary is a second application consuming only the capability-oriented `zui::app`, `zui::task`, `zui::services`, `zui::window`, and `zui::ui` public namespaces. It opens two independently rendered framework-owned windows, runs a scoped background task and event-loop timer, installs an application menu, tray item and global shortcut where supported, accepts `zui-demo://` launch URLs, publishes the frame through AccessKit, and uses the default wgpu backend through `zui` while keeping the default headless test path unchanged.

Verification:

```text
cargo test -p zui-demo
cargo run -p zui-demo
cargo check -p zui-demo --features native --bin zui-native-demo
cargo run -p zui-demo --features native --bin zui-native-demo
```
