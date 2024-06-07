# Changelog

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
