#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

: "${ZETERM_PACKAGE_DIR:?set ZETERM_PACKAGE_DIR to the empty output directory}"
: "${ZETERM_PLATFORM:?set ZETERM_PLATFORM to darwin, linux, or windows}"

set -- --package-dir "${ZETERM_PACKAGE_DIR}"
if [ -n "${ZETERM_TARGET:-}" ]; then
    set -- "$@" --target "${ZETERM_TARGET}"
fi
if [ -n "${ZETERM_CARGO_PROFILE:-}" ]; then
    set -- "$@" --cargo-profile "${ZETERM_CARGO_PROFILE}"
fi

if [ -n "${ZETERM_BINARY:-}" ]; then
    python3 "${script_dir}/build_zeterm_package.py" "$@" --zeterm-bin "${ZETERM_BINARY}"
else
    python3 "${script_dir}/build_zeterm_package.py" "$@"
fi

python3 "${script_dir}/sign_zeterm_package.py" \
    --package-dir "${ZETERM_PACKAGE_DIR}" \
    --platform "${ZETERM_PLATFORM}"
python3 "${script_dir}/verify_zeterm_package.py" \
    --package-dir "${ZETERM_PACKAGE_DIR}" \
    --platform "${ZETERM_PLATFORM}"

echo "Verified release package at ${ZETERM_PACKAGE_DIR}"
