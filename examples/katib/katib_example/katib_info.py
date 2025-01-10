import os
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Self


@dataclass
class KatibInfo:
    base_url: str
    namespace: str
    experiment_name: str
    trial_name: str
    # TODO: Add when https://github.com/kubeflow/katib/issues/2474 is resolved.
    # trial_url: str

    @property
    def experiment_url(self) -> str:
        return f"{self.base_url}/katib/experiment/{self.experiment_name}"

    @classmethod
    def from_env(cls, env: Mapping[str, str] = os.environ) -> Self | None:
        base_url = env.get("KATIB_BASE_URL")
        namespace = env.get("KATIB_NAMESPACE")
        trial_name = env.get("KATIB_TRIAL_NAME")
        if base_url is None and namespace is None and trial_name is None:
            return None
        if base_url is None or namespace is None or trial_name is None:
            raise KeyError(
                "expected all environment variables `KATIB_BASE_URL`, `KATIB_NAMESPACE` and `KATIB_TRIAL_NAME`"
                " to be set when any one of them is set"
            )
        if not base_url:
            raise ValueError("environment variable `KATIB_BASE_URL` may not be empty")
        if not namespace:
            raise ValueError("environment variable `KATIB_NAMESPACE` may not be empty")
        if not trial_name:
            raise ValueError("environment variable `KATIB_TRIAL_NAME` may not be empty")
        experiment_name = trial_name.rsplit("-", 1)[0]
        if not experiment_name:
            raise ValueError("environment variable `KATIB_TRIAL_NAME` must contain experiment name")

        return cls(
            base_url=base_url,
            namespace=namespace,
            experiment_name=experiment_name,
            trial_name=trial_name,
        )

    def tags(self) -> dict[str, str]:
        return {
            "katib.namespace": self.namespace,
            "katib.experiment.name": self.experiment_name,
            "katib.experiment.url": self.experiment_url,
            "katib.trial.name": self.trial_name,
        }
