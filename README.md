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

To install `launch` from source you need to have a recent stable Rust toolchain installed to compile the source code.
We recommend using [`rustup`](https://rustup.rs/) to install a Rust toolchain.

Once you have a Rust toolchain set up, you can install `launch` directly from the git repository with:

```
cargo install --git "https://github.com/Astera-org/obelisk"
```

You can specify a `--branch <branch>`, `--tag <tag>`, or `--rev <revision>` to install a specific version, see the [cargo install documentation](https://doc.rust-lang.org/cargo/commands/cargo-install.html) for details.

Alternatively, if you have checked out this repository or otherwise made the source code available locally, you can install `launch` with:

```
cargo install --path launch
```

To update `launch`, simply `cargo install` another version.

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

## Development

Install the stable and nightly Rust toolchains through [rustup](https://rustup.rs/) with:

```
# headless install of rustup and stable toolchain
curl https://sh.rustup.rs -sSf | sh -s -- -y

# source changes to $PATH into your current shell
. "$HOME/.cargo/env"

# install nightly toolchain
rustup install nightly
```

Now you should be able to build and run `launch`:

```
cargo run --bin launch
```

Format, lint and test your code by running `./ci.sh`.

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

## Release process

1. modify the `version` field in [`launch/Cargo.toml`](./launch/Cargo.toml)
2. create a branch `git checkout -b launch/release-<version>`
3. modify `CHANGELOG.md`:
   1. replace `## Unreleased` with:
        ````md
        ## [<version>] - <yyyy>-<mm>-<dd>

        You can install this version through pixi with:

        ```
        pixi global install --channel https://repo.prefix.dev/obelisk launch==<version>
        ```

        Alternatively, download the appropriate binary for your platform from [GitHub](https://github.com/Astera-org/obelisk/releases/tag/launch/<version>) or build it from source.
        ````
   4. replace:
       ```
       [unreleased]: https://github.com/Astera-org/obelisk/compare/launch/<previous-version>...HEAD
       ```
       with:
       ```
       [<version>]: https://github.com/Astera-org/obelisk/compare/launch/<previous-version>...launch/<version>
       ```
4. commit the changes `git commit -m "Release launch-<version>"`
5. tag the commit `git tag launch/<version>`
6. modify `CHANGELOG.md`:
   1. add above the last release:
        ```
        ## Unreleased

        ### Features

        ### Fixes

        ```
   2. add above the links list:
        ```
        [unreleased]: https://github.com/Astera-org/obelisk/compare/launch/<version>...HEAD
        ```
7. commit the changes `git commit -m "Prepare changelog"`
8. push the changes and tags `git push -u origin launch/release-<version> && git push launch/<version>`
9. merge the release branch back into master
10. post in the `#infra` slack channel:
    ```
    launch <version> has been released :partying_face:. Please view the release page if you use launch.
    ```
    where  `release page` is linked to `https://github.com/Astera-org/obelisk/releases/tag/launch%2F<version>`
