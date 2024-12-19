use std::{error::Error, fmt, thread, time};

use kubernetes::models as k8s;
use log::{debug, info, warn};

use super::{ExecutionArgs, Result};
use crate::kubectl::{self, PodStatus};

pub const KANIKO_POST_BUILD_TIMEOUT: time::Duration = time::Duration::from_secs(30);
pub const RAY_JOB_CREATION_TIMEOUT: time::Duration = time::Duration::from_secs(600);
pub const LOG_AVAILABILITY_TIMEOUT: time::Duration = time::Duration::from_secs(600);
pub const POLLING_INTERVAL: time::Duration = time::Duration::from_secs(2);

pub struct Deadline(time::Instant);

impl Deadline {
    /// Create a new deadline that times out after the provided duration.
    pub fn after(timeout: time::Duration) -> Self {
        Self(time::Instant::now() + timeout)
    }

    /// If there is enough time to sleep before the deadline, sleeps and returns
    /// Ok. Otherwise, returns Err.
    pub fn sleep(&self, duration: time::Duration) -> Result<(), ()> {
        if time::Instant::now() + duration < self.0 {
            thread::sleep(duration);
            Ok(())
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub enum PodLogPollError {
    BadStatus(Box<PodStatus>),
    Timeout,
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for PodLogPollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PodLogPollError::BadStatus(status) => write!(
                f,
                "Pod logs will not become available because it reached status {}",
                status.display_multi_line(0),
            ),
            PodLogPollError::Timeout => write!(
                f,
                "Deadline exceeded while waiting for pod logs to become available!"
            ),
            PodLogPollError::Other(e) => e.fmt(f),
        }
    }
}

impl Error for PodLogPollError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PodLogPollError::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<Box<dyn Error + Send + Sync>> for PodLogPollError {
    fn from(error: Box<dyn Error + Send + Sync>) -> Self {
        PodLogPollError::Other(error)
    }
}

pub fn wait_for_and_follow_pod_logs(
    kubectl: &kubectl::Kubectl,
    namespace: &str,
    name: &str,
) -> Result<(), PodLogPollError> {
    fn log_status(status: &kubectl::PodStatus) {
        debug!("Pod status: {status}");
    }

    info!("Waiting for logs of Pod {namespace}/{name} to become available...");

    let deadline = Deadline::after(LOG_AVAILABILITY_TIMEOUT);
    let mut status = kubectl.pod(namespace, name)?.status;
    log_status(&status);
    loop {
        if let Some(logs_available) = status.are_logs_available() {
            if logs_available {
                break;
            } else if status.is_unschedulable() {
                warn!("The Pod is unschedulable which means that the Pod is queued. The Pod will start once the cluster has sufficient capacity. Please ensure that your Pod does not request more resources than the cluster can possibly offer.");
                return Ok(());
            } else {
                return Err(PodLogPollError::BadStatus(status.into()));
            }
        }

        deadline
            .sleep(POLLING_INTERVAL)
            .map_err(|_| PodLogPollError::Timeout)?;

        status = {
            let new_status = kubectl.pod(namespace, name)?.status;
            if new_status != status {
                log_status(&new_status);
            }
            new_status
        }
    }

    kubectl.follow_pod_logs(namespace, name)?;

    Ok(())
}

pub(super) const PRIMARY_CONTAINER_NAME: &str = "main";

pub(super) fn job_spec(
    args: &ExecutionArgs,
    container_command: Option<Vec<String>>,
    container_args: Option<Vec<String>>,
) -> k8s::V1Job {
    let annotations = args.annotations();

    k8s::V1Job {
        api_version: Some("batch/v1".to_owned()),
        kind: Some("Job".to_owned()),
        metadata: Some(Box::new(k8s::V1ObjectMeta {
            annotations: Some(annotations.clone()),
            generate_name: Some(args.generate_name.to_owned()),
            namespace: Some(args.job_namespace.to_owned()),
            ..Default::default()
        })),
        spec: Some(Box::new(k8s::V1JobSpec {
            // How many times to retry running the pod and all its containers, should any of them
            // fail.
            backoff_limit: Some(0),
            template: Box::new(k8s::V1PodTemplateSpec {
                metadata: Some(Box::new(k8s::V1ObjectMeta {
                    annotations: Some(annotations.clone()),
                    ..Default::default()
                })),
                spec: Some(Box::new(k8s::V1PodSpec {
                    affinity: args.affinity().map(Box::new),
                    containers: vec![k8s::V1Container {
                        name: PRIMARY_CONTAINER_NAME.to_owned(),
                        command: container_command,
                        args: container_args,
                        env: args.env(),
                        image: Some(args.image.image_url()),
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
