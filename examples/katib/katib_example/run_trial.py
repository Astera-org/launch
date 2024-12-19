import sys
import time
from dataclasses import dataclass
from typing import Self

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

    @classmethod
    def from_args(cls) -> Self:
        """Creates a config from the program arguments and validates it."""
        cfg = draccus.parse(config_class=cls)
        # TODO: Remove ` or cfg.tensorboard_dir == "None"` when https://github.com/dlwh/draccus/issues/25 is fixed.
        if not cfg.tensorboard_dir or cfg.tensorboard_dir == "None":
            sys.exit("--tensorboard_dir is required")
        return cfg

    def __repr__(self) -> str:
        return draccus.dump(self)


def run_experiment_trial(cfg: Config):
    with SummaryWriter(log_dir=cfg.tensorboard_dir) as writer:
        # Optimize something
        loss = cfg.nested.hyperparameter**2

        # Log the loss. The `new_style=True` argument is required for katib due
        # to https://github.com/kubeflow/katib/issues/2466
        writer.add_scalar("loss", loss, global_step=0, new_style=True)


def main():
    cfg = Config.from_args()
    print(cfg)
    run_experiment_trial(cfg)

    # Wait for a bit so that the katib metrics sidecar container has enough time
    # to obtain the main container's pid.
    time.sleep(10)


if __name__ == "__main__":
    main()
