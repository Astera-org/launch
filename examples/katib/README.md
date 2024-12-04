# Katib example

Relevant elements:

* [Dockerfile](Dockerfile) for building an image. In this case it uses UV to install Python dependencies.
* [experiment_spec.yaml](experiment_spec.yaml) that specifies how to run the hyperparameter search.
* [Python entrypoint](katib_example/run_trial.py) that uses [Draccus](https://github.com/dlwh/draccus)
  to specify its config and parse command line args. The structure of the config needs to match
  the names of the parameaters in the experiment spec.

Usage:

```sh
cargo run --bin launch -- submit --katib ./experiment_spec.yaml -- python katib_example/run_trial.py
```
