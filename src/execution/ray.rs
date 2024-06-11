//! The ray on kubernetes rayjob backend implementation.

use log::{debug, info, warn};

use crate::{execution::common, kubectl::ResourceHandle};

use super::{ExecutionArgs, ExecutionBackend, ExecutionOutput, Result};

fn rayjob_spec(args: &ExecutionArgs) -> serde_json::Value {
    let image = args.image();
    let entrypoint = args.command.join(" ");
    serde_json::json!({
        "apiVersion": "ray.io/v1",
        "kind": "RayJob",
        "metadata": {
            "namespace": args.job_namespace,
            "generateName": args.generate_name,
            "annotations": {
                "launched_by_user": args.launched_by_user,
                "launched_by_hostname": args.launched_by_hostname
            }
        },
        "spec": {
            "entrypoint": &entrypoint,
            "entrypointNumGpus": 0,
            "shutdownAfterJobFinishes": true,
            "rayClusterSpec": {
                "headGroupSpec": {
                    "serviceType": "NodePort",
                    "rayStartParams": {
                        "dashboard-host": "0.0.0.0"
                    },
                    "template": {
                        "spec": {
                            "containers": [
                                {
                                    "name": "ray-head",
                                    "image": image,
                                    // Default ports, see https://github.com/ray-project/kuberay/blob/master/ray-operator/config/samples/ray-job.sample.yaml.
                                    "ports": [
                                        {
                                            "containerPort": 6379,
                                            "name": "gcs-server"
                                        },
                                        {
                                            "containerPort": 8265,
                                            "name": "dashboard"
                                        },
                                        {
                                            "containerPort": 10001,
                                            "name": "client"
                                        }
                                    ],
                                }
                            ]
                        }
                    }
                },
                "workerGroupSpecs": [
                    {
                        "replicas": args.workers,
                        "groupName": "small-group",
                        "rayStartParams": {},
                        "template": {
                            "spec": {
                                "containers": [
                                    {
                                        "name": "ray-worker",
                                        "image": image,
                                        "lifecycle": {
                                            "preStop": {
                                                "exec": {
                                                    "command": ["/bin/sh", "-c", "ray stop"]
                                                }
                                            }
                                        },
                                        "volumeMounts": args.volume_mounts,
                                        "resources": {
                                            "limits": {
                                                "nvidia.com/gpu": args.gpus,
                                            }
                                        }
                                    }
                                ],
                                "volumes": args.volumes,
                            }
                        }
                    }
                ]
            }
        }
    })
}

pub struct RayExecutionBackend;

impl ExecutionBackend for RayExecutionBackend {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput> {
        let headlamp_base_url = args.headlamp_base_url;

        let (job_namespace, job_name) = {
            let job_spec = rayjob_spec(&args);
            let ResourceHandle { namespace, name } = args.kubectl.create(&job_spec.to_string())?;
            assert_eq!(args.job_namespace, namespace);
            (namespace, name)
        };

        info!("Created rayjob {:?}.", job_name);

        info!("Waiting for job {:?} to become available...", job_name);

        let deadline = common::Deadline::after(common::JOB_CREATION_TIMEOUT);

        loop {
            debug!("Waiting for job {:?} to exist...", job_name);

            match args.kubectl.try_get_job(&job_namespace, &job_name) {
                Ok(Some(_)) => {
                    break;
                }
                Ok(None) => {
                    // Keep polling.
                }
                Err(error) => return Err(error),
            }

            if deadline.sleep(common::POLLING_INTERVAL).is_err() {
                return Err(format!(
                    "Deadline exceeded while waiting for job {:?} to come into existance",
                    job_name
                )
                .into());
            }
        }

        info!(
            "Created job {:?}.",
            format!("{headlamp_base_url}/c/main/jobs/{job_namespace}/{job_name}")
        );

        let pod_name = {
            let mut pod_names = args.kubectl.get_pods_for_job(&job_namespace, &job_name)?;
            for pod_name in &pod_names {
                info!(
                    "Created pod {:?}.",
                    format!("{headlamp_base_url}/c/main/pods/{job_namespace}/{pod_name}")
                );
            }
            let pod_name = pod_names.pop().ok_or("No pods created for job")?;
            if pod_names.len() > 1 {
                warn!(
                    "Following logs only for pod {:?} and ignoring the others.",
                    format!("{headlamp_base_url}/c/main/pods/{job_namespace}/{pod_name}")
                );
            }
            pod_name
        };

        common::wait_for_and_follow_pod_logs(args.kubectl, &job_namespace, &pod_name)?;

        Ok(ExecutionOutput {})
    }
}
