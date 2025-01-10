#!/usr/bin/env bash

# This script makes CI more likely to pass for this project.

set -euo pipefail

# The version is not pinned here nor in CI so we automatically stay up-to-date. If this causes too much churn we can
# consider pinning the toolchain version.
NIGHTLY=nightly

if ! cargo "+$NIGHTLY" --version >/dev/null 2>&1; then
    if command -v rustup >/dev/null 2>&1; then
        rustup install "$NIGHTLY"
    else
        echo -e "\e[1;33mwarn\e[0;1m:\e[0m unable to automatically install rust $NIGHTLY because rustup is not available, consider installing rustup: https://rustup.rs/"
        echo -e "\e[1;31merror\e[0;1m:\e[0m rust $NIGHTLY is required but not installed"
        exit 1
    fi
fi

cargo clippy --fix --allow-dirty --allow-staged --all-features --all-targets -- -D warnings
cargo "+$NIGHTLY" fmt
cargo test

pushd examples/katib
uv run -- ruff format
uv run -- ruff check --fix
popd
