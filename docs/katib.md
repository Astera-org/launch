# Katib integration

[Katib](https://www.kubeflow.org/docs/components/katib/overview/) is a system for hyperparameter tuning.
It can also be used to run many trials of the same job with different random seeds.

Running `launch submit --katib <path-to-katib-expeirment-spec>` will start a Katib experiment.

[examples/katib](../examples/katib) contains a fully-working example of
code plus an experiment spec that you can use as a starting point.

## Katib<->Human interface

The main elements of Katib are:
* An experiment, defined by an experiment spec. In `launch` this is written
  in a YAML file and passed to the `--katib` flag. This specifies the hyperparameters
  to tune, the number of trials to run, the search algorithm to use, etc.
* A trial, which is a single execution of a program with a given set of hyperparameters.

The experiment spec that launch accepts is a subset of the full Katib experiment spec.
The official documentation for the Katib spec is [here](https://www.kubeflow.org/docs/components/katib/user-guides/hp-tuning/configure-experiment/), and
the definition of the spec that launch accepts is [here](../launch/src/katib.rs).

The main things that the full Katib spec allows that the launch spec does not are:
* `trialTemplate`, since the code in launch constructs that based on the command line arguments.
* `metricsCollectorSpec`, since we only support TensorBoard at the default path.

After submitting an experiment, `launch submit` will print out the URL of the experiment on the Katib UI.

## Katib<->Your training code interface

In order to simplify the user experience, the way that `launch` interacts with Katib requires
a particular interface between your code and Katib.

The two things Katib needs from your code are:

### How to specify hyperparameter values

Your program should accept command line arguments for all of the hyperparameters listed in the
experiment spec. For example, if the experiment spec contains:

```yaml
parameters:
  - name: foo
    ...
```

Then your program should accept a `--foo` command line argument.

An easy way to do this is to use [Draccus](https://github.com/dlwh/draccus) to parse the command line arguments.

### How to collect metrics

Your program will be invoked with a `--tensorboard_dir` argument, and must
write any metrics to that directory listed in the experiment spec.

If you're using Draccus, just add a `tensorboard_dir` field to your config class.

For example, if the experiment spec contains:

```yaml
objective:
  objectiveMetricName: loss
```

Then your program should do something like:

```python
from torch.utils import tensorboard

with tensorboard.SummaryWriter(log_dir=cfg.tensorboard_dir) as writer:
    # Log the loss. The `new_style=True` argument is required for katib due
    # to https://github.com/kubeflow/katib/issues/2466
    writer.add_scalar("loss", loss, global_step=0, new_style=True)
```

> **_NOTE:_** Until https://github.com/kubeflow/katib/issues/2466 is fixed, you must
> pass `new_style=True` to `add_scalar`.
> [jax_loop_utils.metric_writers](https://github.com/Astera-org/jax_loop_utils/)
> does this for you, so consider using that.
