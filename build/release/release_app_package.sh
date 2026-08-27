#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

: "${APP_PACKAGE_DIR:?set APP_PACKAGE_DIR to the empty output directory}"
: "${APP_PLATFORM:?set APP_PLATFORM to darwin, linux, or windows}"

set -- --package-dir "${APP_PACKAGE_DIR}"
if [ -n "${APP_TARGET:-}" ]; then
    set -- "$@" --target "${APP_TARGET}"
fi
if [ -n "${APP_CARGO_PROFILE:-}" ]; then
    set -- "$@" --cargo-profile "${APP_CARGO_PROFILE}"
fi
if [ -n "${APP_REMOTE_RUNTIME_BUNDLE:-}" ]; then
    set -- "$@" --remote-runtime-bundle "${APP_REMOTE_RUNTIME_BUNDLE}"
fi
if [ -n "${APP_REMOTE_RUNTIME_CATALOG_URL:-}" ] || [ -n "${APP_REMOTE_RUNTIME_CATALOG_SHA256:-}" ]; then
    : "${APP_REMOTE_RUNTIME_CATALOG_URL:?set both network Remote runtime catalog variables}"
    : "${APP_REMOTE_RUNTIME_CATALOG_SHA256:?set both network Remote runtime catalog variables}"
    set -- "$@" --remote-runtime-catalog-url "${APP_REMOTE_RUNTIME_CATALOG_URL}"
    set -- "$@" --remote-runtime-catalog-sha256 "${APP_REMOTE_RUNTIME_CATALOG_SHA256}"
fi

if [ -n "${APP_BINARY:-}" ]; then
    python3 -B "${script_dir}/build_app_package.py" "$@" --app-bin "${APP_BINARY}"
else
    python3 -B "${script_dir}/build_app_package.py" "$@"
fi

python3 -B "${script_dir}/sign_app_package.py" \
    --package-dir "${APP_PACKAGE_DIR}" \
    --platform "${APP_PLATFORM}"
python3 -B "${script_dir}/verify_app_package.py" \
    --package-dir "${APP_PACKAGE_DIR}" \
    --platform "${APP_PLATFORM}"

echo "Verified release package at ${APP_PACKAGE_DIR}"
