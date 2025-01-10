import os
import time
from dataclasses import dataclass

import draccus
import mlflow
import mlflow.entities
from databricks.sdk import WorkspaceClient
from torch.utils.tensorboard import SummaryWriter

from katib_example.katib_info import KatibInfo


@dataclass
class NestedConfig:
    hyperparameter: float


@dataclass
class Config:
    """Training Config for Machine Learning"""

    nested: NestedConfig
    tensorboard_dir: str

    def __repr__(self) -> str:
        return draccus.dump(self)


def set_experiment(path: str) -> mlflow.entities.Experiment:
    # Databricks requires that we create parent directories. The MLFlow client
    # does not do this by default.
    parts = path.rsplit("/", 1)
    if len(parts) > 1:
        WorkspaceClient().workspace.mkdirs(parts[0])
    return mlflow.set_experiment(path)


def run_experiment_trial(cfg: Config):
    katib = KatibInfo.from_env()
    assert katib is not None

    assert os.environ["MLFLOW_TRACKING_URI"] == "databricks"

    experiment = set_experiment(f"/Shared/launch-example-katib/{katib.experiment_name}")
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


def main():
    cfg = draccus.parse(config_class=Config)
    print(cfg, flush=True)

    run_experiment_trial(cfg)

    # Wait for a bit so that the katib metrics sidecar container has enough time
    # to obtain the main container's pid.
    time.sleep(10)


if __name__ == "__main__":
    main()
