set working-directory := "zeta-rs"

# Launch Zeta from the current source tree.
zeta *args:
    cargo run --bin zeta -- {{ args }}

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    python3 ../scripts/build_zeta_package.py {{ args }}
