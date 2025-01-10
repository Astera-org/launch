# Katib example

Relevant elements:

* [Dockerfile](Dockerfile) for building an image. In this case it uses UV to install Python dependencies.
* [experiment_spec.yaml](experiment_spec.yaml) that specifies how to run the hyperparameter search.
* [Python entrypoint](katib_example/run_trial.py) which defines the program that is invoked for each Katib Trial.

The entrypoint uses [Draccus](https://github.com/dlwh/draccus) to specify its
  config and parse command line args. The structure of the config needs to:
  * match the names of the parameters in the experiment spec.
  * have a `tensorboard_dir` field that specifies where the TensorBoard logs will be written.

The entrypoint also logs the trial to an MLFlow and tags the run with the Katib
Experiment and Trial names and other metadata based on the environment variables
launch and Katib inject into the container.

Usage:

```sh
cargo run --bin launch -- submit --katib ./experiment_spec.yaml -- python katib_example/run_trial.py
```
