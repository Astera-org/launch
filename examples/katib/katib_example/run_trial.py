import sys
import time
from dataclasses import dataclass

import draccus
from tensorboardX import SummaryWriter


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
    writer = SummaryWriter(logdir=cfg.tensorboard_dir)
    writer.add_scalar("loss", loss, 0)
    writer.close()
