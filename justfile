set working-directory := "zeta-rs"

# Launch Zeta from the current source tree.
zeta *args:
    cargo run --bin zeta -- {{ args }}
