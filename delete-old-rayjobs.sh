#!/usr/bin/env bash

# See https://github.com/Astera-org/obelisk/issues/254

set -euo pipefail

kubectl -n launch get rayjobs -o json \
    | jq -r '.items[] |
    select(
        (.metadata.creationTimestamp < (now - 86400 | todateiso8601))
        and (.status.jobStatus == "SUCCEEDED" or .status.jobStatus == "FAILED")
    ) |
    .metadata.name' \
    | xargs -I {} kubectl -n launch delete rayjob {}
