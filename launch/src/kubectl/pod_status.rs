use std::fmt;

use serde::Deserialize;

/// Partially implements [PodStatus](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core)
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PodStatus {
    /// Current service state of pod. More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-conditions
    #[serde(default)]
    pub conditions: Vec<PodCondition>,

    /// The list has one entry per container in the manifest. More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-and-container-status
    #[serde(default)]
    pub container_statuses: Vec<ContainerStatus>,

    /// A human readable message indicating details about why the pod is in this condition.
    #[serde(default)]
    pub message: Option<String>,

    /// A brief CamelCase message indicating details about why the pod is in this state. e.g. 'Evicted'.
    #[serde(default)]
    pub reason: Option<String>,

    /// The phase of a Pod is a simple, high-level summary of where the Pod is in its lifecycle. The conditions array,
    /// the reason and message fields, and the individual container status arrays contain more detail about the pod's
    /// status. There are five possible phase values: Pending: The pod has been accepted by the Kubernetes system, but
    /// one or more of the container images has not been created. This includes time before being scheduled as well as
    /// time spent downloading images over the network, which could take a while. Running: The pod has been bound to a
    /// node, and all of the containers have been created. At least one container is still running, or is in the process
    /// of starting or restarting. Succeeded: All containers in the pod have terminated in success, and will not be
    /// restarted. Failed: All containers in the pod have terminated, and at least one container has terminated in
    /// failure. The container either exited with non-zero status or was terminated by the system. Unknown: For some
    /// reason the state of the pod could not be obtained, typically due to an error in communicating with the host of
    /// the pod. More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-phase
    pub phase: PodPhase,
}

impl PodStatus {
    pub fn is_unschedulable(&self) -> bool {
        self.conditions.iter().any(|condition| {
            condition.r#type == "PodScheduled"
                && condition.reason.as_deref() == Some("Unschedulable")
        })
    }

    /// Returns `Some(value)` where `value` indicates whether the logs are available if it can be determined from the
    /// current status, and `None` otherwise.
    pub fn are_logs_available(&self) -> Option<bool> {
        if self.is_unschedulable() {
            return Some(false);
        };

        if self
            .container_statuses
            .iter()
            .any(ContainerStatus::cannot_pull_image)
        {
            return Some(false);
        };

        match (&self.phase, self.reason.as_deref()) {
            (_, Some("Unschedulable")) | (PodPhase::Unknown, _) => Some(false),
            (PodPhase::Running, Some("Started"))
            | (PodPhase::Succeeded, _)
            | (PodPhase::Failed, _) => Some(true),
            _ => None,
        }
    }
}

impl fmt::Display for PodStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.phase.fmt(f)?;
        if let Some(message) = self.message.as_ref() {
            f.write_str(": ")?;
            f.write_str(message)?;
        }

        for condition in &self.conditions {
            write!(f, ", condition {}", &condition.r#type)?;
            if let Some(reason) = condition.reason.as_deref() {
                write!(f, " {reason}")?;
            }
            if let Some(message) = condition.message.as_deref() {
                write!(f, ": {message}")?;
            }
        }

        for status in &self.container_statuses {
            let ContainerStatus { name, image, state } = status;
            let (state_name, reason, message) = match state {
                ContainerState::Waiting(state) => {
                    ("waiting", state.reason.as_deref(), state.message.as_deref())
                }
                ContainerState::Running(_) => ("running", None, None),
                ContainerState::Terminated(state) => (
                    "terminated",
                    state.reason.as_deref(),
                    state.message.as_deref(),
                ),
            };
            write!(
                f,
                ", container {name:?} using image {image:?} is {state_name}"
            )?;
            if let Some(reason) = reason {
                write!(f, " because {reason}")?;
            }
            if let Some(message) = message {
                write!(f, ": {message}")?;
            }
        }
        Ok(())
    }
}

/// Partially implements [PodCondition](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podcondition-v1-core)
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PodCondition {
    /// Last time we probed the condition.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_probe_time: Option<time::OffsetDateTime>,

    /// Last time the condition transitioned from one status to another.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_transition_time: Option<time::OffsetDateTime>,

    /// Human-readable message indicating details about last transition.
    #[serde(default)]
    pub message: Option<String>,

    /// Unique, one-word, CamelCase reason for the condition's last transition.
    #[serde(default)]
    pub reason: Option<String>,

    /// Status is the status of the condition. Can be True, False, Unknown. More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-conditions
    pub status: String,

    /// Type is the type of the condition. More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-conditions
    pub r#type: String,
}

// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ContainerStatus {
    /// Name is a DNS_LABEL representing the unique name of the container. Each container in a pod must have a unique name across all container types. Cannot be updated.
    pub name: String,

    /// Image is the name of container image that the container is running. The container image may not match the image used in the PodSpec, as it may have been resolved by the runtime. More info: https://kubernetes.io/docs/concepts/containers/images.
    pub image: String,

    pub state: ContainerState,
}

impl ContainerStatus {
    pub fn cannot_pull_image(&self) -> bool {
        let ContainerState::Waiting(state) = &self.state else {
            return false;
        };
        let Some(reason) = &state.reason else {
            return false;
        };
        matches!(reason.as_str(), "ErrImagePull" | "ImagePullBackOff")
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum ContainerState {
    #[serde(rename = "waiting")]
    Waiting(ContainerStateWaiting),
    #[serde(rename = "running")]
    Running(ContainerStateRunning),
    #[serde(rename = "terminated")]
    Terminated(ContainerStateTerminated),
}

/// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#containerstatewaiting-v1-core
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]

pub struct ContainerStateWaiting {
    /// Message regarding why the container is not yet running.
    #[serde(default)]
    message: Option<String>,

    /// (brief) reason the container is not yet running.
    #[serde(default)]
    reason: Option<String>,
}

/// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#containerstaterunning-v1-core
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStateRunning {
    /// Time at which the container was last (re-)started
    #[serde(with = "time::serde::rfc3339")]
    started_at: time::OffsetDateTime,
}

/// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#containerstateterminated-v1-core
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]

pub struct ContainerStateTerminated {
    /// Container's ID in the format '<type>://<container_id>'
    container_id: String,

    /// Exit status from the last termination of the container
    #[serde(default)]
    exit_code: Option<i32>,

    /// Time at which the container last terminated
    finished_at: time::OffsetDateTime,

    /// Message regarding the last termination of the container
    #[serde(default)]
    message: Option<String>,

    /// (brief) reason from the last termination of the container
    #[serde(default)]
    reason: Option<String>,

    /// Signal from the last termination of the container
    #[serde(default)]
    signal: Option<i32>,

    /// Time at which previous execution of the container started
    #[serde(with = "time::serde::rfc3339")]
    started_at: time::OffsetDateTime,
}

/// Field `phase` of [PodStatus](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core).
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum PodPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

impl fmt::Display for PodPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PodPhase::Pending => "pending",
            PodPhase::Running => "running",
            PodPhase::Succeeded => "succeeded",
            PodPhase::Failed => "failed",
            PodPhase::Unknown => "unknown",
        })
    }
}
