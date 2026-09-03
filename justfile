set working-directory := "."
set positional-arguments

export JUST_SHELL := justfile_directory() / "scripts/just-shell.py"

set shell := ["python3", "-c", 'import os, runpy; runpy.run_path(os.environ["JUST_SHELL"], run_name="__main__")']
set windows-shell := ["python", "-c", 'import os, runpy; runpy.run_path(os.environ["JUST_SHELL"], run_name="__main__")']

python := if os_family() == "windows" { "python" } else { "python3" }

# Format Just, Rust, and first-party Python sources.
fmt:
    {{ python }} -B scripts/format.py

# Check formatting without modifying files.
fmt-check:
    {{ python }} -B scripts/format.py --check

# Run repository-owned Python tests, optionally selecting zeta-code, build, or release.
test-python *args:
    {{ python }} -B scripts/test-python.py {args}

# Build all three product lines from the repository root.
build: build-desktop build-rust

# Build the Electron Desktop product.
build-desktop:
    corepack pnpm --dir zeta-ts build

# Build the root Rust workspace with the locked V8 inputs when required.
build-rust *args:
    {{ python }} -B scripts/cargo.py build --workspace {args}

# Test one Rust package. V8 inputs are configured only when its dependency graph needs them.
test *args:
    {{ python }} -B scripts/cargo.py test -p {args}

# Check one Rust package. V8 inputs are configured only when its dependency graph needs them.
check *args:
    {{ python }} -B scripts/cargo.py check -p {args}

# Run versioned multi-Agent evaluations. Real models require the explicit live subcommand flags.
multi-agent-eval *args:
    {{ python }} -B scripts/cargo.py run -p zeta-multi-agent-evals -- {args}

# Fail once the configuration support window makes a compatibility migration removable.
check-config-migrations:
    {{ python }} -B scripts/cargo.py test -p zeta-config tests::config_migration_support_window_has_no_expired_compatibility -- --exact

# Refresh the checked-in App Server protocol fixtures and generated TypeScript client.
generate-protocol:
    cargo run --quiet -p zeta-app-server-protocol --bin generate_protocol -- fixtures
    corepack pnpm --dir zeta-ts run protocol:generate

# Launch the zeta code TUI product from the current source tree.
zeta *args:
    {{ python }} -B scripts/zeta-code/run.py {args}

# Assemble the complete immutable development package shared by Zeta products.
zeta-package *args:
    node build/zeta-package/prepareDevPackage.ts {args}

# Assemble the complete development package and launch Zeta Code against it.
zeta-package-run *args:
    {{ python }} -B scripts/zeta-code/run_package.py {args}

# Launch the zeta Electron Desktop product.
zeta-desktop:
    corepack pnpm --dir zeta-ts dev

# Launch the pure-Rust app Desktop product.
app:
    {{ python }} -B scripts/cargo.py run -p app

# Check every pure-Rust app target with the locked sandbox-enabled V8 inputs.
app-check:
    {{ python }} -B scripts/cargo.py check -p app --all-targets

# Test every pure-Rust app target with the locked sandbox-enabled V8 inputs.
app-test:
    {{ python }} -B scripts/cargo.py test -p app --all-targets

# Stage an unsigned app package; release CI signs and verifies the staged binary.
app-package *args:
    {{ python }} -B build/release/build_app_package.py {args}

# Build, sign, and verify an app package in a platform release job.
app-release:
    {{ python }} -B build/release/release_app_package.py

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    {{ python }} -B build/release/build_zeta_package.py {args}

[unix]
install:
    rustup show active-toolchain
    cargo fetch
    uv sync --frozen --project scripts

[windows]
install:
    #!powershell.exe -File
    $pwsh = Get-Command pwsh.exe -ErrorAction SilentlyContinue
    if (-not $pwsh) {
        winget install --exact --id Microsoft.PowerShell --source winget --accept-package-agreements --accept-source-agreements
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    rustup show active-toolchain
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo fetch
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    uv sync --frozen --project scripts
    exit $LASTEXITCODE
