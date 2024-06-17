#!/usr/bin/env bash

set -euo pipefail

kubectl -n launch get pods -o json \
    | jq -r '
        .items[]
        | select(.status.conditions[]?.reason == "Unschedulable")
        | .metadata.labels["job-name"]' \
    | sort \
    | uniq \
    | xargs -I {} kubectl -n launch delete job {}
