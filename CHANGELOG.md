# Changelog

## Unreleased

### Changes

## [0.1.9] - 2025-01-04

You can install this version through `pixi` with:

```bash
pixi global install --channel https://repo.prefix.dev/obelisk launch==0.1.9
```

Or build it from source with:

```bash
cargo install launch --locked --force --git https://github.com/Astera-org/obelisk --tag launch/0.1.9
```

Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.9) or build it from source.

### Changes

#### [Monitor and log Katib experiment status](https://github.com/Astera-org/obelisk/issues/730)

Launch now polls the status of Katib experiments after launching them.
State changes to the experiment and the resulting trials are logged with hyperlinks to our Kubernetes dashboard in case something is wrong.
This allows the user to view the logs of the Kubernetes pods started for the Katib trials.

#### [Require `--name-prefix` to start with a lowercase ASCII letter](https://github.com/Astera-org/obelisk/pull/829)

Katib requires the Kubernetes resource names to satisfy [RFC 1035 label names](https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#rfc-1035-label-names) instead of [RFC 1123 label names](https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#dns-label-names).
The only difference between the two is that RFC 1035 label names can not start with a digit.

#### [Truncate long experiment names](https://github.com/Astera-org/obelisk/issues/827)

Launch now automatically truncates long experiment names to a maximum of 40 characters.
A warning is logged when this occurs.

## [0.1.8] - 2024-12-23

You can install this version through pixi with:

```
pixi global install --channel https://repo.prefix.dev/obelisk launch==0.1.8
```

Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.8) or build it from source.

### Features

#### Remove OpenSSL

This dependency is no longer required and should fix installed failures on Linux environments.

## [0.1.7] - 2024-12-19

You can install this version through pixi with:

```
pixi global install --channel https://repo.prefix.dev/obelisk launch==0.1.7
```

Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.7) or build it from source.

### Features

#### Submit hyperparameter searches on [Katib](https://www.kubeflow.org/docs/components/katib/user-guides/)

Basic interface:

```sh
launch submit --katib <path to experiment spec YAML> -- python path/to/my_script.py --my_arg
```

#### Remote image building with [kaniko](https://github.com/GoogleContainerTools/kaniko)

Launch can now build your code inside the cluster, before running it. This can
be faster than using Docker to build and push an image, especially in remote
clusters like voltage-park.

You must commit and push all code in your git repo. Have un-commited changes or
un-pushed commits will result in a warning.

Using the kubernetes example:
```sh
cd launch/examples/kubernetes
launch submit --builder kaniko --context voltage-park -- cat README.md
```

From the research folder:
```sh
cd research/
launch submit --builder kaniko --context voltage-park --gpus 1 -- pytest
```

## [0.1.6] - 2024-09-06

You can install this version through pixi with:

```
pixi global install --channel https://repo.prefix.dev/obelisk launch==0.1.6
```

Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.6) or build it from source.

### Features

#### [Add Voltage Park as a Cluster option](https://github.com/Astera-org/obelisk/issues/480)

You can now select `voltage-park` with the `--context` argument.

To submit a job running on the VoltagePark cluster, issue:

```
launch submit --context voltage-park
```

#### [Add comment to list nodes with GPU information](https://github.com/Astera-org/obelisk/issues/263)

The `launch list` subcommand has been modified to take an optional parameter that specifies what to list.

You can now issue `launch list nodes` to list the cluster nodes names and their GPU information.
The output looks something like this:


```
┌────────────────────────────────┬─────────────────────────┬─────────┬───────────┐
│ node                           ┆ GPU                     ┆ GPU mem ┆ GPU count │
╞════════════════════════════════╪═════════════════════════╪═════════╪═══════════╡
│ legal-jackal.astera-infra.com  ┆ NVIDIA-GeForce-RTX-3090 ┆ 24GiB   ┆ 4         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┤
│ set-falcon.astera-infra.com    ┆ NVIDIA-RTX-A6000        ┆ 47GiB   ┆ 4         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┤
│ wadj-wer.astera-infra.com      ┆                         ┆         ┆           │
└────────────────────────────────┴─────────────────────────┴─────────┴───────────┘
```

#### [Enrich launch list output with pod information](https://github.com/Astera-org/obelisk/issues/258)

The `launch list` subcommand now shows information about the running pods for Job and RayJob resources.

```
┌───────────────────┬─────────────────────┬─────────────────────────────┬────────────────────────────┬─────────────┐
│ name              ┆ created (+02:00)    ┆ Job status                  ┆ RayJob status              ┆ launched by │
╞═══════════════════╪═════════════════════╪═════════════════════════════╪════════════════════════════╪═════════════╡
│ eric-6gfsd        ┆ 2024-07-09 20:13:28 ┆ eric-6gfsd-9zmnl: Running   ┆                            ┆ eric        │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ kevin-zrhm6       ┆ 2024-07-08 17:09:58 ┆                             ┆ Initializing               ┆ kevin       │
│                   ┆                     ┆                             ┆ kevin-zrhm6-raycluster-d6w ┆             │
│                   ┆                     ┆                             ┆ qd-worker-small-group-8jrx ┆             │
│                   ┆                     ┆                             ┆ 6: Pending                 ┆             │
│                   ┆                     ┆                             ┆ kevin-zrhm6-raycluster-d6w ┆             │
│                   ┆                     ┆                             ┆ qd-head-t75hl: Running     ┆             │
│                   ┆                     ┆                             ┆ kevin-zrhm6-raycluster-d6w ┆             │
│                   ┆                     ┆                             ┆ qd-worker-small-group-6p45 ┆             │
│                   ┆                     ┆                             ┆ 5: Pending                 ┆             │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ garymm-xtws5      ┆ 2024-07-04 00:44:53 ┆ Failed:                     ┆ Failed                     ┆ garymm      │
│                   ┆                     ┆ BackoffLimitExceeded        ┆                            ┆             │
│                   ┆                     ┆ garymm-xtws5-hwx68: Failed  ┆                            ┆             │
│                   ┆                     ┆ garymm-xtws5-4h9cq: Failed  ┆                            ┆             │
│                   ┆                     ┆ garymm-xtws5-hvtr9: Failed  ┆                            ┆             │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ garymm-gdjf4      ┆ 2024-07-03 02:07:54 ┆ Complete                    ┆ Complete                   ┆ garymm      │
└───────────────────┴─────────────────────┴─────────────────────────────┴────────────────────────────┴─────────────┘
```

#### [Colored launch list output](https://github.com/Astera-org/obelisk/issues/367)

The `launch list` subcommand output is now colored.

### Fixes

## [0.1.5] - 2024-07-09

You can install this version through pixi with:

```
pixi global install --channel https://repo.prefix.dev/obelisk launch==0.1.5
```

Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.5) or build it from source.

### Features

#### [Allow selecting cluster](https://github.com/Astera-org/obelisk/issues/292)

You can now select which cluster to use with the `--context` argument.
Currently it support two values: `staging` and `berkeley`.
By default `berkeley` is used.

As an example, to list the jobs running on the staging cluster, issue:

```
launch list --context staging
```

#### [Automatically set docker image platform](https://github.com/Astera-org/obelisk/issues/312)

Launch will now pass `--platform linux/amd64` when building docker images because the docker images run on machines with that platform.
This eliminates the need to set the platform in the Dockerfile for machines and operating systems which default to a different platform.

#### [Annotate created resources with launch version](https://github.com/Astera-org/obelisk/issues/293)

Launch will now annotate kubernetes resources that it creates with its version under the label `launch.astera.org/version`.

### Fixes

#### [Increase RayJob and log timeouts to 10m](https://github.com/Astera-org/obelisk/issues/297)

The maximum time we wait for the RayJob submitter pod to become available, and for logs to become available, have changed from 180s to 600s.

Having to wait longer than 10m indicates an issue with our infrastructure that we should address instead.

#### [Fix submit program invocation](https://github.com/Astera-org/obelisk/issues/329)

The arguments `program` and `args` provided to `launch submit -- <program> <args>...` are instantiated as a process and not invoked through a shell by default.

If you need to evaluate something through a shell, use `launch submit -- bash -lc '<script>'`.
Make the argument to `bash -lc` is quoted such that things evaluated in the desired shell (your machine or the worker).
For example, `bash -lc "echo $PATH"` echos the path of your machine, while `bash -lc 'echo $PATH'` echos the path of the worker.

For the kubernetes Job execution backend, the docker container's `ENTRYPOINT` is now left intact, rather than being overwritten by `<program> <args>...`.

For the ray RayJob execution backend, the ray job submitter pod still overrides the `ENTRYPOINT` due to a limitation in ray, but now at least executes through a login shell (`bash -lc`) like the head and worker pods.
This allows placing activation scripts in `.bash_profile`.

#### [Support MLFlow in ray job](https://github.com/Astera-org/obelisk/issues/339)

Using mlflow with `tracking_uri="databricks"` now works with `--workers` larger than 1 as it should.

## [0.1.4] - 2024-06-19

You can install this version through pixi with:

```
pixi global install --channel https://repo.prefix.dev/obelisk launch==0.1.4
```

Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.4) or build it from source.

### Features

#### [Expose commit hash to docker image](https://github.com/Astera-org/obelisk/issues/151)

Launch now passes a build argument named `COMMIT_HASH` when building the docker image.
This argument can be accessed in your `Dockerfile` and re-exposed as an environment variable as follows:

```Dockerfile
ARG COMMIT_HASH
ENV ASTERA_SOURCE_GIT_COMMIT=$COMMIT_HASH
```

Your application can then lookup the environment variable `ASTERA_SOURCE_GIT_COMMIT`.
When using `MLFlow`, it can be used to add the git commit as a tag to the run:

```py
if (value := os.environ.get('ASTERA_SOURCE_GIT_COMMIT')):
    client.set_tag(run.info.run_id, "astera.source.git.commit", value)
```

This allows us to find back the source code that is associated with the run, assuming the code was committed and pushed before issuing the run.

#### [Allow specifying the minimum GPU memory](https://github.com/Astera-org/obelisk/issues/245)

The `launch submit` argument `--gpu-mem <GiB>` allows specifying the minimum GPU memory per worker in gibibytes.

GPU data sheets may report a number that is imprecise.
For example, the NVIDIA RTX A6000 [reports "48 GB" of RAM](https://www.nvidia.com/content/dam/en-zz/Solutions/design-visualization/quadro-product-literature/proviz-print-nvidia-rtx-a6000-datasheet-us-nvidia-1454980-r9-web%20(1).pdf).
However, our cluster has determined that this GPU has 49140 MiB of RAM, which equals 47.99 GiB and 51.53 GB.
Because the `--gpu-mem` value is a minimum, specified in GiB, you need to pass `--gpu-mem 47` if you want the job to be able to run on an NVIDIA RTX A6000.

### Fixes

#### [Unschedulable pod detection](https://github.com/Astera-org/obelisk/issues/248)

Launch now detects unschedulable pods when attempting to follow the logs. The reason why a pod is unschedulable will be
printed. For example, if you request more gpus than are available in the cluster `launch submit --gpus 9000 -- nvidia-smi`, the following error will be printed:

```
error: Pod logs will not become available because it reached status pending, condition PodScheduled Unschedulable: 0/8 nodes are available: 8 Insufficient nvidia.com/gpu. preemption: 0/8 nodes are available: 8 No preemption victims found for incoming pod..
```

#### [Delete jobs after a week instead of one day](https://github.com/Astera-org/obelisk/pull/265)

Kubernetes Job now have their `ttlSecondsAfterFinished` set 1 week.

#### [Hide RayJob created message by default](https://github.com/Astera-org/obelisk/pull/253)

This message was not essential and is now logged at the `debug` level.
The RayJob name was replaced by a headlamp link to the [RayJob](https://berkeley-headlamp.taila1eba.ts.net/c/main/customresources/rayjobs.ray.io/?namespace=launch).

## [0.1.3] - 2024-06-12

### Features

#### [`launch list`](https://github.com/Astera-org/obelisk/issues/229)

The `launch list` command lists regular Jobs and RayJobs running on the kubernetes cluster. The output looks something like this:

```
┌──────────────────────────────┬─────────────────────┬──────────────────────────────┬───────────────┬─────────────┐
│ name                         ┆ created (+02:00)    ┆ Job status                   ┆ RayJob status ┆ launched by │
╞══════════════════════════════╪═════════════════════╪══════════════════════════════╪═══════════════╪═════════════╡
│ launch-eric-z69kq            ┆ 2024-06-11 02:16:51 ┆ Failed: BackoffLimitExceeded ┆               ┆ eric        │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ launch-eric-fwsls            ┆ 2024-06-10 18:27:01 ┆ Failed: BackoffLimitExceeded ┆               ┆ eric        │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ launch-mick-astera-org-phwfg ┆ 2024-06-10 12:41:18 ┆ Complete                     ┆ Complete      ┆ mick        │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ launch-mick-astera-org-695tw ┆ 2024-06-06 07:47:15 ┆ Failed: BackoffLimitExceeded ┆ Failed        ┆             │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ launch-mick-astera-org-mb2s2 ┆ 2024-06-04 18:19:08 ┆                              ┆ Running       ┆             │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ launch-mick-astera-org-xlf4s ┆ 2024-06-04 17:48:04 ┆ Failed: BackoffLimitExceeded ┆ Failed        ┆             │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ launch-mick-astera-org-pj49m ┆ 2024-05-31 01:31:04 ┆                              ┆ Initializing  ┆             │
└──────────────────────────────┴─────────────────────┴──────────────────────────────┴───────────────┴─────────────┘
```

The `Job status` column is only present when a Job with `name` exists, and is derived from its [`status.conditions`](https://github.com/kubernetes/kubernetes/issues/68712) field.
The `RayJob status` column is only present when a RayJob with `name` exists, and is derived from its [`status.jobDeploymentStatus`](https://docs.ray.io/en/latest/cluster/kubernetes/getting-started/rayjob-quick-start.html#step-8-check-the-rayjob-status) field.

#### [Store tailscale user, machine user and machine hostname](https://github.com/Astera-org/obelisk/issues/237)

The Kubernetes resource annotations `launched_by_user` and `launched_by_host` have been removed.
New annotations have been added:

- `launch.astera.org/launched-by-machine-user` which contains `<username>@<hostname>` of the machine submitting work, if it can be determined.
- `launch.astera.org/launched-by-tailscale-user` which contains the Tailscale login name, if it can be determined.

The `launch list` command does not respect the old resource annotations, only the new ones.

#### Failure to determine hostname is no longer an error

If the hostname can not be determined, a warning is printed instead.

#### Failure to determine tailscale login name is no longer an error

If the tailscale login name can not be determined, a warning is printed instead.

#### Resource name template is now `{user}-`

The resource name template changed from `launch-{user_with_hostname}-` to `{user}-`.
If you want, you can specify a prefix with `--name-prefix`. If you do, the resource name template becomes `{prefix}-{user}-`

### Fixes

#### [Per-user databricks secret](https://github.com/Astera-org/obelisk/issues/227)

The databricks secret resource name is now determined based on the user name `databricks-{name}`.
This makes it so that jobs of different users can't incorrectly use secrets that another user overwrote.

## [0.1.2] - 2024-06-07

### Features

#### [Execution with Ray through RayJobs](https://github.com/Astera-org/obelisk/issues/152)

### Fixes
If the number of workers is more than 1, the `ray` execution backend is used.
Note that in order to utilize the workers spawned for the RayJob, you must create work for those workers to run in your entrypoint python script.
An example of this is provided in [examples/ray/](./examples/ray/).
The `--gpus <N>` argument applies to the workers and not to the entrypoint.
The entrypoint always has 0 GPUs in this version of `launch`.

#### Add git version information to docker image

The docker images built with `launch` are now annotated with the git hash.
A warning is issued when there are uncommitted changes or if commits have not yet been pushed to a remote.

[unreleased]: https://github.com/Astera-org/obelisk/compare/launch/0.1.9...HEAD
[0.1.9]: https://github.com/Astera-org/obelisk/compare/launch/0.1.8...launch/0.1.9
[0.1.8]: https://github.com/Astera-org/obelisk/compare/launch/0.1.7...launch/0.1.8
[0.1.7]: https://github.com/Astera-org/obelisk/compare/launch/0.1.6...launch/0.1.7
[0.1.6]: https://github.com/Astera-org/obelisk/compare/launch/0.1.5...launch/0.1.6
[0.1.5]: https://github.com/Astera-org/obelisk/compare/launch/0.1.4...launch/0.1.5
[0.1.4]: https://github.com/Astera-org/obelisk/compare/launch/0.1.3...launch/0.1.4
[0.1.3]: https://github.com/Astera-org/obelisk/compare/launch/0.1.2...launch/0.1.3
[0.1.2]: https://github.com/Astera-org/obelisk/releases/tag/launch/0.1.2
