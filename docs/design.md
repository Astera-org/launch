
## Job Submission

The overall process of job submission is described in the following diagram.

```mermaid
graph TD;
    collect_build_parameters["Collect build parameters (git hash, ...)"] --> build_backend;
    build_backend{"Build Backend"};
    build_backend --"Build Locally"--> local_build["docker buildx build --push"];
    build_backend --"Build Remote"--> remote["supply git commit\nkubectl create -f build-remote.yml"];
    remote --> remote_machine
    subgraph remote_machine["Build Kubernetes Job"]
        remote_pull["git fetch && git checkout &lt; commit &gt;"] --> remote_build;
        remote_build["docker buildx build --push"];
    end
    local_build & remote_machine --"image digest"--> collect_execution_parameters;

    collect_execution_parameters["Collect execution parameters (created by, image digest, ...)"] --> execution_backend;

    execution_backend{Execution Backend};

    execution_backend --"Kubernetes Job"--> execution_initiate_kubernetes;
    execution_initiate_kubernetes["kubectl create -f job.yml"] --> execution_kubernetes;
    subgraph execution_kubernetes["Execution Kubernetes Job"]
        execution_kubernetes_command["Run provided command"]
    end;

    execution_initiate_kubernetes & execution_initial_rayjob --"pod id"--> tail_logs;

    execution_backend --"Ray Job"--> execution_initial_rayjob;
    execution_initial_rayjob["kubectl create -f ray-job.yml"] --> execution_rayjob;
    subgraph execution_rayjob["Execution RayJob"]
        execution_rayjob_command["Run provided command"]
    end;
```

### Design Decisions

#### Should the build job initiate the execution job?

No, we will pull these apart which makes it easier to forward errors to the user and separates responsibility.

#### What should the build job output?

The resulting container image name and digest. Maybe also the registry url, but that url may be different inside/outside the kubernetes cluster.

#### How do we supply credentials?

The execution backends as of now support mounting kubernetes secrets as volumes. We can use that to add secrets from the job submission's machine into the worker pods.

#### Should we require a clean git working directory?

By default we should probably check that:

- the working directory is clean
- the current commit is available on the remote

We can then include the git commit hash.

We should consider allowing the user to opt out through a flag or something like `--allow-dirty`.

#### How do we specify the number of GPUs for the entry point, the number of workers, etc?

- `--build <local|remote> default: remote` to specify the build backend.
- `--execution <kubernetes|ray> default: kubernetes` to specify the execution backend.
- `--workers <N> where N >= 1 default 1` to specify the number of workers.
- `--gpus <N> where N >= 0 default 0` to specify the number of gpus per worker.

Jobs can end up not utilizing all requested GPUs. For RayJobs, we can enable auto scaling to mitigate that somewhat.

#### Should we support spawning multiple workers for the kubernetes execution backend?

Might be useful, the question is how do we tail the logs of multiple workers, just the first one, all mixed together, none by default?
