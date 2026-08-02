set working-directory := "zeta-rs"

# Launch the zeta code TUI product from the current source tree.
zeta *args:
    cargo run --bin zeta -- {{ args }}

# Rebuild and restart the zeta code TUI product when its sources change.
zeta-dev:
    watchexec --restart --exts rs,toml -- cargo run --bin zeta

# Launch the zeta Electron Desktop product.
zeta-desktop:
    corepack pnpm --dir ../desktop dev

# Launch the pure-Rust zeterm Desktop product.
zeterm:
    cargo run -p zeta-native

# Rebuild and restart the zeterm Desktop product when Rust or shader sources change.
zeterm-dev:
    watchexec --restart --exts rs,toml,wgsl -- cargo run -p zeta-native

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    python3 ../scripts/build_zeta_package.py {{ args }}
