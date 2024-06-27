//! Implementation details shared by some execution backends.

use std::{error::Error, fmt, thread, time};

use log::{debug, info};

use super::Result;
use crate::kubectl::{self, PodStatus};

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
    Unschedulable,
    BadStatus(Box<PodStatus>),
    Timeout,
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for PodLogPollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PodLogPollError::Unschedulable => write!(f, "Pod is unschedulable"),
            PodLogPollError::BadStatus(status) => write!(
                f,
                "Pod logs will not become available because it reached status {status}"
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

    info!("Waiting for pod logs to become available...");

    let deadline = Deadline::after(LOG_AVAILABILITY_TIMEOUT);
    let mut status = kubectl.pod_status(namespace, name)?;
    log_status(&status);
    loop {
        if let Some(logs_available) = status.are_logs_available() {
            if logs_available {
                break;
            } else if status.is_unschedulable() {
                return Err(PodLogPollError::Unschedulable);
            } else {
                return Err(PodLogPollError::BadStatus(status.into()));
            }
        }

        deadline
            .sleep(POLLING_INTERVAL)
            .map_err(|_| PodLogPollError::Timeout)?;

        status = {
            let new_status = kubectl.pod_status(namespace, name)?;
            if new_status != status {
                log_status(&new_status);
            }
            new_status
        }
    }

    kubectl.follow_pod_logs(namespace, name)?;

    Ok(())
}
