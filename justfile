set working-directory := "."

# Launch the zeta code TUI product from the current source tree.
zeta *args:
    cargo run -p zeta-cli --bin zeta -- {{ args }}

# Rebuild and restart the zeta code TUI product when its sources change.
zeta-dev:
    watchexec --restart --exts rs,toml -- cargo run -p zeta-cli --bin zeta

# Launch the zeta Electron Desktop product.
zeta-desktop:
    corepack pnpm --dir desktop dev

# Launch the pure-Rust zeterm Desktop product.
zeterm:
    cargo run -p zeterm

# Rebuild and restart the zeterm Desktop product when Rust or shader sources change.
zeterm-dev:
    watchexec --restart --exts rs,toml,wgsl -- cargo run -p zeterm

# Stage an unsigned zeterm package; release CI signs and verifies the staged binary.
zeterm-package *args:
    python3 -B build/release/build_zeterm_package.py {{ args }}

# Build, sign, and verify a zeterm package in a platform release job.
zeterm-release:
    build/release/release_zeterm_package.sh

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    python3 -B build/release/build_zeta_package.py {{ args }}
