//! Implementation details shared by some execution backends.

use std::{thread, time};

use log::{debug, info};

use super::Result;
use crate::kubectl;

pub const JOB_CREATION_TIMEOUT: time::Duration = time::Duration::from_secs(180);
pub const LOG_AVAILABILITY_TIMEOUT: time::Duration = time::Duration::from_secs(180);
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

pub fn wait_for_and_follow_pod_logs(
    kubectl: &kubectl::Kubectl,
    namespace: &str,
    name: &str,
) -> Result<()> {
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
            }
            return Err(format!(
                "Pod logs will not become available because it reached status {status}"
            )
            .into());
        }

        deadline.sleep(POLLING_INTERVAL).map_err(|_| {
            "Deadline exceeded while waiting for pod logs to become available!".to_string()
        })?;

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
