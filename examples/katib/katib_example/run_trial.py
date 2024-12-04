import time
from dataclasses import dataclass

import draccus


@dataclass
class NestedConfig:
    hyperparameter: float


@dataclass
class Config:
    """Training Config for Machine Learning"""

    nested: NestedConfig


def run_trial(cfg: Config) -> float:
    time.sleep(10)  # katib seems to fail if the trial worker exits too quickly
    return cfg.nested.hyperparameter**2


if __name__ == "__main__":
    cfg = draccus.parse(config_class=Config)
    loss = run_trial(cfg)
    # Assumes Katib is using standard output metric collector
    print(f"loss={loss}")
