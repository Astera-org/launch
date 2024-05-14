#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

info() {
    echo -e "\033[1;34mINFO:\033[0m $1"
}

warn() {
    echo -e "\033[1;33mWARNING:\033[0m $1"
}

err() {
    echo -e "\033[1;31mERROR:\033[0m $1"
}

fail() {
    err "$1"
    exit 1
}

does_command_exist() {
    command -v "$1" >/dev/null 2>&1
}

find_tailscale() {
    if does_command_exist tailscale; then
        echo "tailscale"
    elif [ "$(uname -s)" = "Darwin" ] && [ -x "/Applications/Tailscale.app/Contents/MacOS/Tailscale" ]; then
        echo "/Applications/Tailscale.app/Contents/MacOS/Tailscale"
    else
        echo ""
    fi
}

assert_command_exists() {
    local _command="$1"
    if ! does_command_exist "$_command"; then
        fail "\`$_command\` was not found on your system. Please install it."
    fi
}

# Configuration variables
DOCKER_REGISTRY="berkeley-docker.taila1eba.ts.net"
HEADLAMP_BASE_URL="https://berkeley-headlamp.taila1eba.ts.net"
JOB_NAMESPACE="obelisk-launch"
GPU_COUNT="0"

# Ensure that the programs required by this script are available.
assert_command_exists docker
assert_command_exists kubectl
TAILSCALE=$(find_tailscale)
[[ -z "$TAILSCALE" ]] && fail "\`tailscale\` was not found on your system. Please install Tailscale (https://tailscale.com/download)."

[[ -f "$HOME/.databrickscfg" ]] || fail "Databricks config not found. Please configure databricks using the instructions in \`fluid/README.md\`."

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "$SCRIPT_DIR/../fluid"
# NOTE: The build may take more than 5 minutes when pixi dependencies are modified but generally
# should complete within a second.
docker build . -t "$DOCKER_REGISTRY/fluid:latest" || exit 1

docker push "$DOCKER_REGISTRY/fluid:latest" || exit 1

# FIXME: We should at least restore the previous kubernetes context when the script is done.
# Configure and activate our kubernetes context.
"${TAILSCALE}" configure kubeconfig berkeley-tailscale-operator

kubectl delete secret databrickscfg "--namespace=$JOB_NAMESPACE" --ignore-not-found || exit 1

kubectl create secret generic databrickscfg "--namespace=$JOB_NAMESPACE" --from-file="$HOME/.databrickscfg" || exit 1

LAUNCHED_BY_USER=$("$TAILSCALE" status | head -n 1 | sed -E 's/^[^ ]+ *[^ ]+ *([^ ]+)@.*$/\1@astera.org/' | tr -d '\n')
JOB=$(< "$SCRIPT_DIR/job.yml")
JOB="${JOB//\{\{JOB_NAMESPACE\}\}/$JOB_NAMESPACE}"
JOB="${JOB//\{\{LAUNCHED_BY_USER\}\}/$LAUNCHED_BY_USER}"
JOB="${JOB//\{\{LAUNCHED_BY_HOSTNAME\}\}/$(hostname)}"
JOB="${JOB//\{\{GPU_COUNT\}\}/$GPU_COUNT}"

# Create the job and capture the output
CREATE_OUTPUT=$(kubectl create -f - <<< $JOB) || exit 1
echo "$CREATE_OUTPUT"

# Extract the generated job name from the output
JOB_NAME=$(sed 's/job\.batch\/\(.*\) created/\1/' <<< "$CREATE_OUTPUT")
info "Job Name: \"$JOB_NAME\""
info "Job URL: \"$HEADLAMP_BASE_URL/c/main/jobs/$JOB_NAMESPACE/$JOB_NAME\""

# Get the pod name
info "Querying pods for job \"$JOB_NAME\"..."
POD_NAME=$(kubectl get pods "--namespace=$JOB_NAMESPACE" "--selector=job-name=$JOB_NAME" --output=jsonpath='{.items[*].metadata.name}')
info "Pod Name: \"$POD_NAME\""
info "Pod URL: \"$HEADLAMP_BASE_URL/c/main/pods/$JOB_NAMESPACE/$POD_NAME\""

# NOTE: `kubectl wait` does not work when the container command fails, so we just poll.
info "Waiting for logs to become available for pod \"$POD_NAME\"..."
for (( attempt=1; attempt<=100; attempt++ )); do
    sleep 2
    if kubectl logs "--namespace=$JOB_NAMESPACE" -f "$POD_NAME"; then
        info "Finished streaming logs from pod."

        # Describing the pod after the logs finish helps notice that the pod fails to schedule
        # (surprisingly streaming the logs of a unschedulable pods finishes with exit code 0).
        info "Describing pod..."
        kubectl describe pod "--namespace=$JOB_NAMESPACE" "$POD_NAME"

        exit 0
    fi
done

warn "Giving up waiting for the logs. View logs with \`kubectl logs "--namespace=$JOB_NAMESPACE" -f \"$POD_NAME\"\`."
