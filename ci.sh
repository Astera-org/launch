#!/usr/bin/env bash

# Make the code likely to pass CI.

set -euo pipefail

cargo clippy --fix --allow-dirty --allow-staged
cargo fmt
cargo test
