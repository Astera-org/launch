import mlflow
from mlflow.utils.databricks_utils import get_databricks_host_creds
import ray
from ray.runtime_env import RuntimeEnv


@ray.remote
def work(x):
    print(f"I am running on a worker, doubling {x}!")
    mlflow.set_tracking_uri("databricks")
    mlflow.log_metric("x", x)
    return x * 2


def main():
    print("I am the entrypoint!")
    mlflow.set_tracking_uri("databricks")
    mlflow.set_experiment("/Shared/launch_example_ray_pixi")
    with mlflow.start_run() as mlflow_run:
        run_id = mlflow_run.info.run_id
        experiment_id = mlflow_run.info.experiment_id

        databricks_host = (
            creds.host if (creds := get_databricks_host_creds()) is not None else None
        )
        assert databricks_host is not None
        run_uri = f"{databricks_host}/ml/experiments/{experiment_id}/runs/{run_id}"
        print("Run:", run_uri)

        runtime_env = RuntimeEnv(env_vars={"MLFLOW_RUN_ID": run_id})
        with ray.init(runtime_env=runtime_env):
            futures = [work.remote(i) for i in range(2)]
            print("results from remote tasks:")
            print(ray.get(futures))


if __name__ == "__main__":
    main()
