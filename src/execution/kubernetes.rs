//! The kubernetes job backend implementation.

use super::{ExecutionArgs, ExecutionBackend, ExecutionOutput, Result};
use crate::{execution::common, kubectl::ResourceHandle};
use log::info;

fn job_spec(args: &ExecutionArgs) -> serde_json::Value {
    let image = args.image();
    let annotations = args.annotations();
    let volume_mounts = args.volume_mounts();
    let volumes = args.volumes();
    serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "namespace": args.job_namespace,
            "generateName": args.generate_name,
            "annotations": annotations,
        },
        "spec": {
            "template": {
                "metadata": {
                    "annotations": annotations,
                },
                "spec": {
                    "containers": [
                        {
                            "name": "main",
                            "image": &image,
                            "command": args.command,
                            "env": [
                                {
                                    // Suppress warnings from GitPython (used by mlflow)
                                    // about the git executable not being available.
                                    "name": "GIT_PYTHON_REFRESH",
                                    "value": "quiet"
                                }
                            ],
                            "volumeMounts": volume_mounts,
                            "resources": {
                                "limits": {
                                    "nvidia.com/gpu": args.gpus,
                                }
                            }
                        }
                    ],
                    "volumes": volumes,
                    // Defines whether a container should be restarted until it 1) runs forever, 2)
                    // runs succesfully, or 3) has run once. We just want our command to run once
                    // and so we never restart.
                    "restartPolicy": "Never"
                }
            },
            // How many times to retry running the pod and all its containers, should any of them
            // fail.
            "backoffLimit": 0,
            "ttlSecondsAfterFinished": 86400
        }
    })
}

pub struct KubernetesExecutionBackend;

impl ExecutionBackend for KubernetesExecutionBackend {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput> {
        let headlamp_base_url = args.headlamp_base_url;

        let (job_namespace, job_name) = {
            let job_spec = job_spec(&args);
            let ResourceHandle { namespace, name } = args.kubectl.create(&job_spec.to_string())?;
            assert_eq!(args.job_namespace, namespace);
            (namespace, name)
        };

        info!(
            "Created Job {:?}",
            format!("{headlamp_base_url}/c/main/jobs/{job_namespace}/{job_name}")
        );

        let pod_name = {
            let mut pod_names = args.kubectl.get_pods_for_job(&job_namespace, &job_name)?;
            for pod_name in &pod_names {
                info!(
                    "Created Pod {:?}",
                    format!("{headlamp_base_url}/c/main/pods/{job_namespace}/{pod_name}")
                );
            }
            let pod_name = pod_names.pop().ok_or("No pods created for job")?;
            if !pod_names.is_empty() {
                return Err(format!(
                    "Expected only a single Pod for Job {job_name:?} but there are multiple. Not sure for which one to follow the logs."
                )
                .into());
            }
            pod_name
        };

        common::wait_for_and_follow_pod_logs(args.kubectl, &job_namespace, &pod_name)?;

        Ok(ExecutionOutput {})
    }
}
