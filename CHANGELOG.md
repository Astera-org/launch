# Changelog

## Unreleased

### Features

#### Add headlamp link for created RayJobs

When a job with the ray execution backend is created, the [RayJob link](https://berkeley-headlamp.taila1eba.ts.net/c/main/customresources/rayjobs.ray.io/?namespace=launch) is printed instead of just the RayJob name.

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

There are now two methods of executing work on the cluster: regular Kubernetes jobs and [RayJobs](https://docs.ray.io/en/master/cluster/kubernetes/getting-started/rayjob-quick-start.html) (which are different from Ray jobs...).
The `kubernetes` execution backend is used if the number of workers, which can be specified with `--workers <N>`, is 1.
If the number of workers is more than 1, the `ray` execution backend is used.
Note that in order to utilize the workers spawned for the RayJob, you must create work for those workers to run in your entrypoint python script.
An example of this is provided in [examples/ray/](./examples/ray/).
The `--gpus <N>` argument applies to the workers and not to the entrypoint.
The entrypoint always has 0 GPUs in this version of `launch`.

#### Add git version information to docker image

The docker images built with `launch` are now annotated with the git hash.
A warning is issued when there are uncommitted changes or if commits have not yet been pushed to a remote.

### Fixes

None

[unreleased]: https://github.com/Astera-org/obelisk/compare/launch-0.1.3...HEAD
[0.1.3]: https://github.com/Astera-org/obelisk/compare/launch-0.1.2...launch-0.1.3
[0.1.2]: https://github.com/Astera-org/obelisk/releases/tag/launch-0.1.2
