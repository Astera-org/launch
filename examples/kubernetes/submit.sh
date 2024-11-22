#!/usr/bin/env bash

set -euo pipefail

pushd "$( dirname -- "${BASH_SOURCE[0]}")"

cargo run --bin launch -- \
    submit --gpus 1 -- \
    /bin/bash -c "echo \"testing workspace contents uploaded: \$(wc -c README.md)\" && echo \"testing GPU:\" && nvidia-smi"
