# Changelog

## Unreleased

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

[unreleased]: https://github.com/Astera-org/obelisk/compare/launch-0.1.2...HEAD
[0.1.2]: https://github.com/Astera-org/obelisk/releases/tag/launch-0.1.2
