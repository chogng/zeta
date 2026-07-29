set working-directory := "zeta-rs"

# Launch Zeta from the current source tree.
zeta *args:
    cargo run --bin zeta -- {{ args }}

# Launch the Ratatui product.
tui:
    cargo run --bin zeta

# Rebuild and restart the Ratatui product when its sources change.
tui-dev:
    watchexec --restart --exts rs,toml -- cargo run --bin zeta

# Launch the native text-first product shell.
native:
    cargo run -p zeta-native

# Rebuild and restart the native product when Rust or shader sources change.
native-dev:
    watchexec --restart --exts rs,toml,wgsl -- cargo run -p zeta-native

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    python3 ../scripts/build_zeta_package.py {{ args }}
