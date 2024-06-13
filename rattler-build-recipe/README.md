# Rattler Build Recipe

See [rattler-build](https://prefix-dev.github.io/rattler-build/latest/) docs.

Usage:

You should normally just bump the version in Cargo.toml, and then a GitHub workflow
will automatically build and upload the package.
But to run locally:

```sh
pixi global install rattler-build
cd "$(git rev-parse --show-toplevel)/launch"
rattler-build build --experimental --output-dir target/rattler-build --recipe rattler-build-recipe/recipe.yaml
```
