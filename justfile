set working-directory := "."

# Launch the zeta code TUI product from the current source tree.
zeta *args:
    python3 -B build/cargo_with_v8.py run -p zeta-cli --bin zeta -- {{ args }}

# Rebuild and restart the zeta code TUI product when its sources change.
zeta-dev:
    watchexec --restart --exts rs,toml -- python3 -B build/cargo_with_v8.py run -p zeta-cli --bin zeta

# Launch the zeta Electron Desktop product.
zeta-desktop:
    corepack pnpm --dir zeta-ts dev

# Launch the pure-Rust app Desktop product.
app:
    python3 -B build/cargo_with_v8.py run -p app

# Rebuild and restart the app Desktop product when Rust or shader sources change.
app-dev:
    watchexec --restart --exts rs,toml,wgsl -- python3 -B build/cargo_with_v8.py run -p app

# Check every pure-Rust app target with the locked sandbox-enabled V8 inputs.
app-check:
    python3 -B build/cargo_with_v8.py check -p app --all-targets

# Test every pure-Rust app target with the locked sandbox-enabled V8 inputs.
app-test:
    python3 -B build/cargo_with_v8.py test -p app --all-targets

# Stage an unsigned app package; release CI signs and verifies the staged binary.
app-package *args:
    python3 -B build/release/build_app_package.py {{ args }}

# Build, sign, and verify a app package in a platform release job.
app-release:
    build/release/release_app_package.sh

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    python3 -B build/release/build_zeta_package.py {{ args }}
