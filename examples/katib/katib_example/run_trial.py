"""Entrypoint to run an experiment trial."""

import os
import time
from dataclasses import dataclass

import draccus
import mlflow
import mlflow.entities
from databricks.sdk import WorkspaceClient
from torch.utils.tensorboard.writer import SummaryWriter

from katib_example.launch_katib import KatibTrialInfo


@dataclass
class NestedConfig:
    """Some nested configuration."""

    hyperparameter: float


@dataclass
class Config:
    """Training Config for Machine Learning."""

    nested: NestedConfig
    tensorboard_dir: str

    def __repr__(self) -> str:
        return draccus.cfgparsing.dump(self)


def _set_experiment(path: str) -> mlflow.entities.Experiment:
    # Databricks requires that we create parent directories. The MLFlow client
    # does not do this by default.
    parts = path.rsplit("/", 1)
    if len(parts) > 1:
        WorkspaceClient().workspace.mkdirs(parts[0])
    return mlflow.set_experiment(path)


def _run_experiment_trial(cfg: Config):
    katib = KatibTrialInfo.from_env()
    assert katib is not None

    assert os.environ["MLFLOW_TRACKING_URI"] == "databricks"

    experiment = _set_experiment(f"/Shared/launch-example-katib/{katib.experiment_name}")
    with mlflow.start_run(experiment_id=experiment.experiment_id, run_name=katib.trial_name) as run:
        mlflow.set_tags(katib.tags())

        with SummaryWriter(log_dir=cfg.tensorboard_dir) as writer:
            writer.add_hparams(
                {
                    "nested__hyperparameter": cfg.nested.hyperparameter,
                },
                {},
            )
            mlflow.log_params(
                {
                    "nested__hyperparameter": cfg.nested.hyperparameter,
                }
            )

            # Optimize something
            loss = cfg.nested.hyperparameter**2

            # Log the loss. The `new_style=True` argument is required for katib due
            # to https://github.com/kubeflow/katib/issues/2466
            writer.add_scalar("loss", loss, global_step=0, new_style=True)
            mlflow.log_metrics({"loss": loss}, step=0, run_id=run.info.run_id)


def _main():
    cfg = draccus.argparsing.parse(config_class=Config)
    print(cfg, flush=True)

    _run_experiment_trial(cfg)

    # Wait for a bit so that the katib metrics sidecar container has enough time
    # to obtain the main container's pid.
    time.sleep(10)


if __name__ == "__main__":
    _main()
