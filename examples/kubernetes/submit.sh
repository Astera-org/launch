#!/usr/bin/env bash

set -euo pipefail

pushd "$( dirname -- "${BASH_SOURCE[0]}")"

cargo run --bin launch -- submit -- cat README.md
