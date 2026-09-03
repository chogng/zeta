set working-directory := "."
set positional-arguments
export JUST_SHELL := justfile_directory() / "scripts/just-shell.py"
set shell := ["python3", "-c", 'import os, runpy; runpy.run_path(os.environ["JUST_SHELL"], run_name="__main__")']
set windows-shell := ["python", "-c", 'import os, runpy; runpy.run_path(os.environ["JUST_SHELL"], run_name="__main__")']

python := if os_family() == "windows" { "python" } else { "python3" }

# Test one Rust package. V8 inputs are configured only when its dependency graph needs them.
test *args:
    {{ python }} -B build/cargo_with_v8.py test -p {args}

# Check one Rust package. V8 inputs are configured only when its dependency graph needs them.
check *args:
    {{ python }} -B build/cargo_with_v8.py check -p {args}

# Run versioned multi-Agent evaluations. Real models require the explicit live subcommand flags.
multi-agent-eval *args:
    {{ python }} -B build/cargo_with_v8.py run -p zeta-multi-agent-evals -- {args}

# Fail once the configuration support window makes a compatibility migration removable.
check-config-migrations:
    {{ python }} -B build/cargo_with_v8.py test -p zeta-config tests::config_migration_support_window_has_no_expired_compatibility -- --exact

# Launch the zeta code TUI product from the current source tree.
zeta *args:
    {{ python }} -B build/cargo_with_v8.py build -p zeta-app-server-daemon --bin zeta-app-server-daemon
    {{ python }} -B build/cargo_with_v8.py run -p zeta-cli --bin zeta -- {args}

# Launch the zeta Electron Desktop product.
zeta-desktop:
    corepack pnpm --dir zeta-ts dev

# Launch the pure-Rust app Desktop product.
app:
    {{ python }} -B build/cargo_with_v8.py run -p app

# Check every pure-Rust app target with the locked sandbox-enabled V8 inputs.
app-check:
    {{ python }} -B build/cargo_with_v8.py check -p app --all-targets

# Test every pure-Rust app target with the locked sandbox-enabled V8 inputs.
app-test:
    {{ python }} -B build/cargo_with_v8.py test -p app --all-targets

# Stage an unsigned app package; release CI signs and verifies the staged binary.
app-package *args:
    {{ python }} -B build/release/build_app_package.py {args}

# Build, sign, and verify a app package in a platform release job.
[unix]
app-release:
    build/release/release_app_package.sh

# Build a canonical Zeta package; pass normal build_zeta_package.py flags.
package *args:
    {{ python }} -B build/release/build_zeta_package.py {args}

[unix]
install:
    rustup show active-toolchain
    cargo fetch

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
    exit $LASTEXITCODE
