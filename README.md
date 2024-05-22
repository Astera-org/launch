# launch

Launch makes it easy to run a program in our kubernetes cluster. It works by building a docker container and creating a kubernetes job using that container.

## Prerequisites

The following applications should be available on your system to build `launch`.

- [Rust Toolchain](https://rustup.rs/) to build `launch`.

The following applications should be available on your system to run `launch`.

- [Tailscale](https://astera.getoutline.com/doc/tailscale-vpn-SJAKxvmBuw) to access our services.
- [Docker Engine](https://docs.docker.com/engine/) (is included in [Docker Desktop](https://docs.docker.com/desktop/)) to build the docker container.
- [kubectl](https://kubernetes.io/docs/tasks/tools/) to interact with our cluster.

You should also [set up databricks](../fluid/README.md) on your machine. The databricks authentication information from your machine is injected into the docker container running in kubernetes.

## Building

To make `launch` available on your machine, run:

```
git checkout master
git pull
cargo install --path launch
```

Now you should be able to run `launch`.

## Updating

To update `launch`, repeat the steps under [building](#building).

## Running

To view the help, simply run:

```
launch --help
```

Since `launch` creates a pod which requires a docker container, the directory from which you invoke `launch` needs to have a `Dockerfile`. In this repository, that means changing directory to `<repo>/fluid/`.

To run your work on a pod in the cluster, while in `fluid/`:

```
launch submit -- <command> <args...>
```

For example, to change the working directory to `fluid/fluid/`, print the python version and the working directory, you can run:

```
launch submit -- bash -c 'eval "$(pixi shell-hook)" && cd fluid && python --version && pwd'
```

> NOTE: The first time you build the docker image it will install all of the dependencies. This may take 4 minutes. The resulting image is about 9 GB in size and the first push to our docker registry may take 10 minutes. The first time a node pulls the image from our registry may also be slow and the pod will be in the `ContainerCreating` state for a few minutes. Subsequent runs are almost instant assuming the dependencies are not modified.

To learn how to specify the number of GPUs among other things, browse the help by running:

```
launch submit --help
```

## Debugging

To see what commands `launch` is running, and other useful information, set the `RUST_LOG` environment variable to `debug`. More advanced specifications than `debug` are possible.

For example:

`RUST_LOG=debug launch`
