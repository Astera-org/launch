//! The kubernetes job backend implementation.

use kubernetes::models as km;
use log::info;

use super::{ExecutionArgs, ExecutionOutput, Executor, Result};
use crate::{
    executor::common::{self},
    kubectl::ResourceHandle,
};

fn job_spec(args: &ExecutionArgs) -> km::V1Job {
    let annotations = args.annotations();

    km::V1Job {
        api_version: Some("batch/v1".to_owned()),
        kind: Some("Job".to_owned()),
        metadata: Some(Box::new(km::V1ObjectMeta {
            annotations: Some(annotations.clone()),
            generate_name: Some(args.generate_name.to_owned()),
            namespace: Some(args.job_namespace.to_owned()),
            ..Default::default()
        })),
        spec: Some(Box::new(km::V1JobSpec {
            // How many times to retry running the pod and all its containers, should any of them
            // fail.
            backoff_limit: Some(0),
            template: Box::new(km::V1PodTemplateSpec {
                metadata: Some(Box::new(km::V1ObjectMeta {
                    annotations: Some(annotations.clone()),
                    ..Default::default()
                })),
                spec: Some(Box::new(km::V1PodSpec {
                    affinity: args.affinity().map(Box::new),
                    containers: vec![km::V1Container {
                        name: "main".to_owned(),
                        // Using args rather than command keeps the ENTRYPOINT intact.
                        args: Some(args.command.to_owned()),
                        env: args.env(),
                        image: Some(args.image()),
                        volume_mounts: args.volume_mounts(),
                        resources: args.resources().map(Box::new),
                        ..Default::default()
                    }],
                    restart_policy: Some("Never".to_owned()),
                    volumes: args.volumes(),
                    ..Default::default()
                })),
            }),
            ttl_seconds_after_finished: Some(7 * 24 * 3600),
            ..Default::default()
        })),
        ..Default::default()
    }
}

pub struct KubernetesExecutionBackend;

impl Executor for KubernetesExecutionBackend {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput> {
        let kubectl = args.context.kubectl();
        let headlamp_url = args.context.headlamp_url();

        let (job_namespace, job_name) = {
            let job_spec = job_spec(&args);
            let ResourceHandle { namespace, name } =
                kubectl.create(&serde_json::to_string(&job_spec)?)?;
            assert_eq!(args.job_namespace, namespace);
            (namespace, name)
        };

        info!(
            "Created Job {:?}",
            format!("{headlamp_url}/c/main/jobs/{job_namespace}/{job_name}")
        );

        let pod_name = {
            let mut pod_names = kubectl.get_pods_for_job(&job_namespace, &job_name)?;
            for pod_name in &pod_names {
                info!(
                    "Created Pod {:?}",
                    format!("{headlamp_url}/c/main/pods/{job_namespace}/{pod_name}")
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

        common::wait_for_and_follow_pod_logs(&kubectl, &job_namespace, &pod_name)?;

        Ok(ExecutionOutput {})
    }
}
