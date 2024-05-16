# obelisk-launch

Obelisk launch makes it easy to run a program in our kubernetes cluster. It works by building a docker container and creating a kubernetes job using that container.

## Prerequisites

The following applications should be available on your system.

- [Tailscale](https://astera.getoutline.com/doc/tailscale-vpn-SJAKxvmBuw) to access our services.
- [Docker Engine](https://docs.docker.com/engine/) (is included in [Docker Desktop](https://docs.docker.com/desktop/)) to build the docker container.
- [kubectl](https://kubernetes.io/docs/tasks/tools/) to interact with our cluster.

You should also [set up databricks](../fluid/README.md) on your machine. The databricks authentication information from your machine is injected into the docker container running in kubernetes.

## Running

To run your work on a pod in the cluster, from the repository root:

```
cd fluid
../launch/launch.sh
```

You can modify the command that is run in [`job.yml`](./job.yml) by adjusting the `command` property.
The number of GPUs to request can be set in [`launch.sh`](./launch.sh) by modifying the `GPU_COUNT` variable.
To change the working directory, you can use a command like this:

```
command:
- "bash"
- "-c"
- 'cd fluid && eval "$(pixi shell-hook)" && python --version && pwd'
```

This changes the working directory to "fluid", activates the pixi shell, prints the python version and the current working directory.

> NOTE: The first time you build the docker image it will install all of the dependencies. This may take 4 minutes. The resulting image is about 9 GB in size and the first push to our docker registry may take 10 minutes. Subsequent runs are almost instant assuming the dependencies are not modified.
