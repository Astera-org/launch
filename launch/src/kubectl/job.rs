use std::collections::HashMap;

use serde::Deserialize;

use super::ResourceMetadata;

#[derive(Debug, Deserialize)]
pub struct Job {
    pub metadata: ResourceMetadata,
    pub status: JobStatus,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// https://kubernetes.io/docs/reference/kubernetes-api/workload-resources/job-v1/#JobStatus
///
/// The latest available observations of an object's current state. When a Job fails, one of the conditions will have
/// type "Failed" and status true. When a Job is suspended, one of the conditions will have type "Suspended" and status
/// true; when the Job is resumed, the status of this condition will become false. When a Job is completed, one of the
/// conditions will have type "Complete" and status true. More info:
/// https://kubernetes.io/docs/concepts/workloads/controllers/jobs-run-to-completion/
pub struct JobStatus {
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub start_time: Option<time::OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub completion_time: Option<time::OffsetDateTime>,
    #[serde(default)]
    pub ready: Option<u64>,
    #[serde(default)]
    pub active: Option<u64>,
    #[serde(default)]
    pub failed: Option<u64>,
    #[serde(default)]
    pub succeeded: Option<u64>,
    #[serde(default)]
    pub conditions: Vec<JobCondition>,
    #[serde(default)]
    pub uncounted_terminated_pods: HashMap<String, String>,
}

/// [JobCondition](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#jobcondition-v1-batch)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobCondition {
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_probe_time: Option<time::OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_transition_time: Option<time::OffsetDateTime>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(with = "job_condition_status")]
    pub status: bool,
    pub r#type: JobConditionType,
}

pub mod job_condition_status {
    // Learn more at https://serde.rs/custom-date-format.html.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match serde::Deserialize::deserialize(deserializer)? {
            "True" => true,
            "False" => false,
            invalid => {
                return Err(serde::de::Error::unknown_variant(
                    invalid,
                    &["True", "False"],
                ))
            }
        })
    }
}

#[derive(Debug, Deserialize)]
pub enum JobConditionType {
    Failed,
    Suspended,
    Complete,
}

impl JobConditionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobConditionType::Failed => "Failed",
            JobConditionType::Suspended => "Suspended",
            JobConditionType::Complete => "Complete",
        }
    }
}
