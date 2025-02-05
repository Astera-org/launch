//! The ray on kubernetes ray_job backend implementation.

use log::{debug, info, warn};

use super::{ExecutionArgs, ExecutionOutput, Executor, Result};
use crate::{bash_escape, executor::common, kubectl::ResourceHandle};

fn ray_job_spec(args: &ExecutionArgs) -> serde_json::Value {
    let annotations = args.annotations();

    // Ray parses this string with `shlex`. See https://github.com/Astera-org/obelisk/issues/329.
    let entrypoint = bash_escape::quote_join(args.container_args.iter().map(String::as_str));

    serde_json::json!({
        "apiVersion": "ray.io/v1",
        "kind": "RayJob",
        "metadata": {
            "namespace": args.job_namespace,
            "generateName": args.generate_name,
            "annotations": annotations,
        },
        "spec": {
            "entrypoint": entrypoint,
            "shutdownAfterJobFinishes": true,
            "rayClusterSpec": {
                "headGroupSpec": {
                    "serviceType": "NodePort",
                    "rayStartParams": {
                        "dashboard-host": "0.0.0.0",
                        // To prevent workloads with CPU requirements from being scheduled on the head.
                        // See https://docs.ray.io/en/latest/cluster/kubernetes/user-guides/config.html#num-cpus
                        "num-cpus": "0",
                    },
                    "template": {
                        "metadata": {
                            "annotations": annotations,
                        },
                        "spec": {
                            "containers": [
                                {
                                    "name": "ray-head",
                                    "image": args.image,
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
                                    "volumeMounts": args.volume_mounts(),
                                    "env": args.env(),
                                }
                            ],
                            "volumes": args.volumes(),
                        }
                    }
                },
                "workerGroupSpecs": [
                    {
                        "replicas": args.workers,
                        "groupName": "small-group",
                        "rayStartParams": {},
                        "template": {
                            "metadata": {
                                "annotations": annotations,
                            },
                            "spec": {
                                "affinity": args.affinity(),
                                "containers": [
                                    {
                                        "name": "ray-worker",
                                        "image": args.image,
                                        "lifecycle": {
                                            "preStop": {
                                                "exec": {
                                                    // Modified to use bash with a login shell to use ray from PATH set in .bash_profile.
                                                    // TODO: this doesn't seem to work reliably. https://github.com/Astera-org/obelisk/issues/341
                                                    "command": ["/bin/bash", "-lc", "--", "ray stop"]
                                                }
                                            }
                                        },
                                        "resources": args.resources(),
                                        "volumeMounts": args.volume_mounts(),
                                        "env": args.env(),
                                    }
                                ],
                                "volumes": args.volumes(),
                            }
                        }
                    }
                ],
            },
            "submitterPodTemplate": {
                "metadata": {
                    "annotations": annotations,
                },
                "spec": {
                    "restartPolicy": "Never",
                    "containers": [
                        {
                            "name": "ray-job-submitter",
                            "image": args.image,
                            // We have to specify the command because otherwise kuberay overwrites it. Ideally, we would
                            // omit this and use `args` instead. See https://github.com/ray-project/kuberay/pull/2208.
                            "command": ["/bin/bash", "-lc", "--"],
                            // We should not quote this script. The script contains the quoted entrypoint. See https://github.com/Astera-org/obelisk/issues/329.
                            "args": [format!("ray job submit --address=http://$RAY_DASHBOARD_ADDRESS --submission-id=$RAY_JOB_SUBMISSION_ID -- {entrypoint}")],
                        }
                    ]
                }
            }
        }
    })
}

pub struct RayExecutor;

impl Executor for RayExecutor {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput> {
        let kubectl = args.context.kubectl();
        let headlamp_url = args.context.headlamp_url();

        let (job_namespace, job_name) = {
            let job_spec = ray_job_spec(&args);
            let ResourceHandle { namespace, name } = kubectl.create(&job_spec.to_string())?;
            assert_eq!(args.job_namespace, namespace);
            (namespace, name)
        };
        debug!(
            "Created RayJob {:?}.",
            format!(
                "{headlamp_url}/c/main/customresources/rayjobs.ray.io/{job_namespace}/{job_name}"
            )
        );

        let deadline = common::Deadline::after(common::RAY_JOB_CREATION_TIMEOUT);

        info!(
            "Waiting for submitter Job {:?} to become available...",
            job_name
        );

        loop {
            match kubectl.try_get_job(&job_namespace, &job_name) {
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

            debug!(
                "Waiting for submitter Job {:?} to become available...",
                job_name
            );
        }

        info!(
            "Created submitter Job {:?}.",
            format!("{headlamp_url}/c/main/jobs/{job_namespace}/{job_name}")
        );

        let pod_name = {
            let mut pod_names = kubectl.get_pods_for_job(&job_namespace, &job_name)?;
            for pod_name in &pod_names {
                info!(
                    "Created submitter Pod {:?}.",
                    format!("{headlamp_url}/c/main/pods/{job_namespace}/{pod_name}")
                );
            }
            let pod_name = pod_names.pop().ok_or("No pods created for job")?;
            if pod_names.len() > 1 {
                warn!(
                    "Following logs only for Pod {:?} and ignoring the others.",
                    format!("{headlamp_url}/c/main/pods/{job_namespace}/{pod_name}")
                );
            }
            pod_name
        };

        common::wait_for_and_follow_pod_logs(&kubectl, &job_namespace, &pod_name)?;

        Ok(ExecutionOutput {})
    }
}
