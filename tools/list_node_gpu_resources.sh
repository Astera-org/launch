#!/usr/bin/env bash

# Prints something like the following:
#
# node                            gpu_allocatable  gpu_capacity
# clever-jaguar.astera-infra.com  1                1
# legal-jackal.astera-infra.com   4                4
# prompt-iguana.astera-infra.com
# quick-weevil.astera-infra.com   4                4
# set-falcon.astera-infra.com     4                4
#
# Requires the permission to list nodes.

kubectl get nodes -o=json | jq -r '
  .items
  | map({
    node: .metadata.name,
    gpu_allocatable: .status.allocatable."nvidia.com/gpu",
    gpu_capacity: .status.capacity."nvidia.com/gpu",
  })
  | (.[0] | to_entries | map(.key)), (.[] | [.[]])
  | @tsv' | column -t -s $'\t'
