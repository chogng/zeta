# `zeta-code`

`zeta-code` is the product-owned source boundary for the `zeta code` terminal product.
It contains the `zeta-cli` command host and the `zeta-tui` presentation shell.

The product depends on shared contracts and runtime services from `zeta-rs`, but the terminal
experience itself does not belong to the shared backend. Raw mode, alternate-screen lifecycle,
keyboard input, Ratatui layout, composer state, and TUI presentation state stay here.

```text
zeta-code/cli  → zeta-code/tui → zeta-app-server-client → shared App Server crates
```

The native `zeterm` product has a separate ownership boundary under `zeterm/`; its reusable GPU UI
crates, including `zeta-ui` and `zui`, must not be copied into this product.

Run the product from the repository root:

```bash
cargo run --manifest-path Cargo.toml -p zeta-cli
cargo test --manifest-path Cargo.toml -p zeta-tui
bazel test //zeta-code/tui:tui-unit-tests
```
