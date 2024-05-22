use serde::{
    de::{self, Deserializer, MapAccess, Visitor},
    Deserialize,
};
use std::{collections::HashMap, fmt};

// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core
#[derive(Debug, PartialEq)]
pub struct PodStatus {
    pub phase: PodPhase,
    pub message: Option<String>,
    pub container_statuses: Vec<ContainerStatus>,
}

// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core
#[derive(Debug, Deserialize, PartialEq)]
pub struct ContainerStatus {
    pub name: String,
    pub image: String,
    pub state: HashMap<String, ContainerState>,
}

impl ContainerStatus {
    pub fn cannot_pull_image(&self) -> bool {
        let Some(waiting) = self.state.get("waiting") else {
            return false;
        };
        let Some(reason) = &waiting.reason else {
            return false;
        };
        matches!(reason.as_str(), "ErrImagePull" | "ImagePullBackOff")
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ContainerState {
    message: Option<String>,
    reason: Option<String>,
}

impl fmt::Display for PodStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.phase.fmt(f)?;
        if let Some(message) = self.message.as_ref() {
            f.write_str(": ")?;
            f.write_str(message)?;
        }
        for status in &self.container_statuses {
            let ContainerStatus { name, image, state } = status;
            let (state_name, ContainerState { message, reason }) = {
                let mut state_iter = state.iter();
                let first = state_iter.next().unwrap();
                assert!(state_iter.next().is_none());
                first
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

// NOTE(mickvangelderen): I'm not too sure this structure is correct, but it works for now. We might
// want to just deserialize a struct with fields { phase: Phase, reason: Option<String>, message:
// Option<String> because that will be a bit easier to maintain than the deserialization code here.
#[derive(Debug, PartialEq, Deserialize)]
pub enum PodPhase {
    Pending(Option<PodPhasePendingReason>),
    Running(Option<PodPhaseRunningReason>),
    Succeeded(Option<PodPhaseSucceededReason>),
    Failed(Option<PodPhaseFailedReason>),
    Unknown(Option<String>),
}

impl fmt::Display for PodPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PodPhase::Pending(reason) => {
                f.write_str("pending")?;
                if let Some(reason) = reason.as_ref() {
                    write!(f, " ({reason})")?;
                }
            }
            PodPhase::Running(reason) => {
                f.write_str("running")?;
                if let Some(reason) = reason.as_ref() {
                    write!(f, " ({reason})")?;
                }
            }
            PodPhase::Succeeded(reason) => {
                f.write_str("succeeded")?;
                if let Some(reason) = reason.as_ref() {
                    write!(f, " ({reason})")?;
                }
            }
            PodPhase::Failed(reason) => {
                f.write_str("failed")?;
                if let Some(reason) = reason.as_ref() {
                    write!(f, " ({reason})")?;
                }
            }
            PodPhase::Unknown(reason) => {
                f.write_str("unknown")?;
                if let Some(reason) = reason.as_ref() {
                    write!(f, " ({reason})")?;
                }
            }
        }
        Ok(())
    }
}

// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core
#[derive(Debug, PartialEq, Deserialize)]
pub enum PodPhasePendingReason {
    ContainerCreating,
    PodScheduled,
    Unschedulable,
}

impl fmt::Display for PodPhasePendingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PodPhasePendingReason::ContainerCreating => f.write_str("container creating"),
            PodPhasePendingReason::PodScheduled => f.write_str("pod scheduled"),
            PodPhasePendingReason::Unschedulable => f.write_str("unschedulable"),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub enum PodPhaseRunningReason {
    Started,
    ContainerCreating,
    PodInitializing,
}

impl fmt::Display for PodPhaseRunningReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            PodPhaseRunningReason::Started => "started",
            PodPhaseRunningReason::ContainerCreating => "container creating",
            PodPhaseRunningReason::PodInitializing => "pod initializing",
        };
        f.write_str(str)
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub enum PodPhaseSucceededReason {
    Completed,
}

impl fmt::Display for PodPhaseSucceededReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            PodPhaseSucceededReason::Completed => "completed",
        };
        f.write_str(str)
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub enum PodPhaseFailedReason {
    Error,
    Evicted,
    DeadlineExceeded,
}

impl fmt::Display for PodPhaseFailedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            PodPhaseFailedReason::Error => "error",
            PodPhaseFailedReason::Evicted => "evicted",
            PodPhaseFailedReason::DeadlineExceeded => "deadline exceeded",
        };
        f.write_str(str)
    }
}

impl<'de> Deserialize<'de> for PodStatus {
    fn deserialize<D>(deserializer: D) -> Result<PodStatus, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(PodStatusVisitor)
    }
}

struct PodStatusVisitor;

impl<'de> Visitor<'de> for PodStatusVisitor {
    type Value = PodStatus;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with phase and reason fields")
    }

    fn visit_map<V>(self, mut map: V) -> Result<PodStatus, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut phase: Option<&str> = None;
        let mut reason: Option<&str> = None;
        let mut message: Option<String> = None;
        let mut container_statuses: Option<Vec<ContainerStatus>> = None;

        while let Some(key) = map.next_key()? {
            match key {
                "phase" => {
                    if phase.is_some() {
                        return Err(de::Error::duplicate_field("phase"));
                    }
                    phase = Some(map.next_value()?);
                }
                "reason" => {
                    if reason.is_some() {
                        return Err(de::Error::duplicate_field("reason"));
                    }
                    reason = Some(map.next_value()?);
                }
                "message" => {
                    if message.is_some() {
                        return Err(de::Error::duplicate_field("message"));
                    }
                    message = Some(map.next_value()?);
                }
                "containerStatuses" => {
                    if container_statuses.is_some() {
                        return Err(de::Error::duplicate_field("containerStatuses"));
                    }
                    container_statuses = Some(map.next_value()?);
                }
                _ => {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let phase = phase.ok_or_else(|| de::Error::missing_field("phase"))?;

        let phase = match phase {
            "Pending" => PodPhase::Pending(
                reason
                    .map(|reason| match reason {
                        "ContainerCreating" => Ok(PodPhasePendingReason::ContainerCreating),
                        "PodScheduled" => Ok(PodPhasePendingReason::PodScheduled),
                        "Unschedulable" => Ok(PodPhasePendingReason::Unschedulable),
                        _ => Err(de::Error::unknown_variant(
                            reason,
                            &["ContainerCreating", "PodScheduled", "Unschedulable"],
                        )),
                    })
                    .transpose()?,
            ),
            "Running" => PodPhase::Running(
                reason
                    .map(|reason| match reason {
                        "Started" => Ok(PodPhaseRunningReason::Started),
                        "ContainerCreating" => Ok(PodPhaseRunningReason::ContainerCreating),
                        "PodInitializing" => Ok(PodPhaseRunningReason::PodInitializing),
                        _ => Err(de::Error::unknown_variant(
                            reason,
                            &["Started", "ContainerCreating", "PodInitializing"],
                        )),
                    })
                    .transpose()?,
            ),
            "Succeeded" => PodPhase::Succeeded(
                reason
                    .map(|reason| match reason {
                        "Completed" => Ok(PodPhaseSucceededReason::Completed),
                        _ => Err(de::Error::unknown_variant(reason, &["Completed"])),
                    })
                    .transpose()?,
            ),
            "Failed" => PodPhase::Failed(
                reason
                    .map(|reason| match reason {
                        "Error" => Ok(PodPhaseFailedReason::Error),
                        "Evicted" => Ok(PodPhaseFailedReason::Evicted),
                        "DeadlineExceeded" => Ok(PodPhaseFailedReason::DeadlineExceeded),
                        _ => Err(de::Error::unknown_variant(
                            reason,
                            &["Error", "Evicted", "DeadlineExceeded"],
                        )),
                    })
                    .transpose()?,
            ),
            "Unknown" => PodPhase::Unknown(reason.map(str::to_string)),
            _ => {
                return Err(de::Error::unknown_variant(
                    phase,
                    &["Pending", "Running", "Succeeded", "Failed", "Unknown"],
                ))
            }
        };

        Ok(PodStatus {
            phase,
            message,
            container_statuses: container_statuses.unwrap_or_default(),
        })
    }
}
