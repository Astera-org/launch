//! The kubernetes job backend implementation.

use log::info;

use super::{ExecutionArgs, ExecutionOutput, Executor, Result};
use crate::{
    executor::common::{self, job_spec},
    kubectl::ResourceHandle,
};

pub struct KubernetesExecutor;

impl Executor for KubernetesExecutor {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput> {
        let kubectl = args.context.kubectl();
        let headlamp_url = args.context.headlamp_url();

        let (job_namespace, job_name) = {
            let job_spec = job_spec(&args, None, Some(args.container_args.to_vec()));
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
