#!/usr/bin/env bash

# Prints something like the following:
#
# namespace                     pod                                           node                            gpu_request  status     creation_date            launched_by_user  launched_by_hostname
# gh-actions-runner-scale-sets  berkeley-gpu-runners-c8wpl-runner-8g7br       clever-jaguar.astera-infra.com  1            Running    2024-05-13 16:49:48 PST  N/A               N/A
# launch                        obelisk-fluid-9gf2r-dxwl9                     set-falcon.astera-infra.com     1            Succeeded  2024-05-13 15:36:01 PST  N/A               pop-os
# launch                        obelisk-fluid-xj4hb-lrfsk                     set-falcon.astera-infra.com     1            Succeeded  2024-05-13 14:49:21 PST  N/A               pop-os
# ray                           ray-cluster-kuberay-worker-workergroup-9gbh8  set-falcon.astera-infra.com     4            Running    2024-05-13 15:38:26 PST  N/A               N/A
# ray                           ray-cluster-kuberay-worker-workergroup-r7zrg  legal-jackal.astera-infra.com   4            Running    2024-05-13 15:39:35 PST  N/A               N/A

kubectl get pods --all-namespaces -o json | jq -r '
  .items
  | map(
    select(any(.spec.containers[].resources.requests."nvidia.com/gpu"; . != null and (. | tonumber) >= 1))
    | {
      namespace: .metadata.namespace,
      pod: .metadata.name,
      node: .spec.nodeName,
      gpu_request: (.spec.containers[].resources.requests."nvidia.com/gpu" // "0"),
      status: .status.phase,
      creation_date: (.metadata.creationTimestamp | fromdateiso8601 | strflocaltime("%Y-%m-%d %H:%M:%S %Z")),
      launched_by_user: (.metadata.annotations["launched_by_user"] // "N/A"),
      launched_by_hostname: (.metadata.annotations["launched_by_hostname"] // "N/A")
    }
  )
  | (.[0] | to_entries | map(.key)), (.[] | [.[]])
  | @tsv' | column -t -s $'\t'
