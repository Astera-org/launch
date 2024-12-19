import sys
import time
from dataclasses import dataclass

import draccus
from torch.utils.tensorboard import SummaryWriter


@dataclass
class NestedConfig:
    hyperparameter: float


@dataclass
class Config:
    """Training Config for Machine Learning"""

    nested: NestedConfig
    tensorboard_dir: str


def run_trial(cfg: Config) -> float:
    time.sleep(10)  # katib seems to fail if the trial worker exits too quickly
    return cfg.nested.hyperparameter**2


if __name__ == "__main__":
    cfg = draccus.parse(config_class=Config)
    loss = run_trial(cfg)
    # Assumes Katib is using tensorflow event metric collector
    if not cfg.tensorboard_dir:
        sys.exit("--tensorboard_dir is required")
    writer = SummaryWriter(log_dir=cfg.tensorboard_dir)
    # new_style required for katib due to
    # https://github.com/kubeflow/katib/issues/2466
    writer.add_scalar("loss", loss, global_step=0, new_style=True)
    writer.close()
