# launch

Launch makes it easy to run a program in our kubernetes cluster. It works by building a docker container and creating a kubernetes job using that container.

## Installation

You will need to both 1) obtain the `launch` binary, 2) and ensure that certain programs which `launch` needs when you run it are also installed.

### Installing `launch` itself

You can obtain the `launch` program itself as a pre-built binary or you can build it from source.

#### From private registry

Launch is available as a package in our [private registry](https://repo.prefix.dev/obelisk). To authenticate with our private registry, use the ["prefix.dev API key" from 1password](https://asterainstitute.1password.com/vaults/5fznj7lifbm3qqmwvv6mde6upm/allitems/yesuo2guat53a7riiv4n7kcpya) in place of `<token>` and run:

```
pixi auth login https://repo.prefix.dev/obelisk --token <token>
```

To install `launch`, run:

```
pixi global install -c https://repo.prefix.dev/obelisk launch
```

To update `launch`, run:

```
pixi global upgrade launch
```

#### From source

To build `launch` from source you need to [install a recent stable Rust toolchain](https://rustup.rs/).

To build and make `launch` available on your machine, run:

```
git checkout master
git pull
cargo install --path launch
```

To update `launch`, repeat the above steps.

### Installing `launch`'s run-time dependencies

The following applications should be available on your system in order to run `launch`.

| Name                                                         | Installation Check    | Installation Instructions                                                                                                               |
| ------------------------------------------------------------ | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| [Tailscale](https://tailscale.com/kb/1151/what-is-tailscale) | `tailscale --version` | [instructions](https://astera.getoutline.com/doc/tailscale-vpn-SJAKxvmBuw)                                                              |
| [Kubernetes](https://kubernetes.io/docs/concepts/overview/)  | `kubectl version`     | [instructions](https://kubernetes.io/docs/tasks/tools/)                                                                                 |
| [Docker](https://docs.docker.com/engine/)                    | `docker --version`    | [Docker Engine](https://docs.docker.com/engine/) (Docker Engine is also included in [Docker Desktop](https://docs.docker.com/desktop/)) |
| [Git](https://git-scm.com/)                                  | `git --version`       | [instructions](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)                                                           |

### **\[Optional\]** Authenticate with databricks

If your jobs use MLFlow, your job needs a file that allows MLFlow to push information to databricks.
Once per machine, follow the [steps described in the fluid README](../fluid/README.md#logging-to-mlflow).
The databricks authentication information from your machine will then be injected into the containers running your jobs.

## Usage

To view the help, simply run:

```
launch --help
```

There are a number of subcommands, one of which is `submit` which allows you to submit work to our cluster.
You can view the help of a subcommand with:

```
launch <subcommand> --help
```

Please use the help text from the `launch` application, it should be very informative.

## Examples

The [`examples/`](./examples/) folder contains some standalone projects which can be used to try out `launch`.
View the `README.md` inside an example project for more information.

## Debugging

There are many steps involved in running work on the cluster.
If something does not work, try to determine if 1) the job submission itself is failing, or 2) the command that you are supplying does not run succesfully inside the docker container.

### Understanding what `launch` is doing

To see what commands `launch` is running under the hood and other useful information, set the `RUST_LOG` environment variable to `debug`:

```
RUST_LOG=debug launch
```

More [advanced specifications](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) than `debug` are possible if you need finer control of what is being logged.

### Running the docker image locally

It can be helpful to run a docker container locally if your command does not run successfully.
By running a docker container locally, you can inspect the file system inside the container or try out some commands quickly.
To do so, determine the docker image digest from the build output emitted by `launch`.
It should look something like:

```
 => => writing image sha256:89b7200c2632bdf418a6bc10f8a26495ab929947c6d962833a9114310df15532
```

Then run that image with:

```
docker run --rm -it sha256:89b7200c2632bdf418a6bc10f8a26495ab929947c6d962833a9114310df15532
```
