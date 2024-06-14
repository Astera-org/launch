use serde::Deserialize;
use std::collections::HashMap;

use super::ResourceMetadata;

#[derive(Debug, Deserialize)]
pub struct Job {
    pub metadata: ResourceMetadata,
    pub status: JobStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// https://kubernetes.io/docs/reference/kubernetes-api/workload-resources/job-v1/#JobStatus
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
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub uncounted_terminated_pods: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub last_probe_time: String,
    pub last_transition_time: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    pub status: String,
    pub r#type: String,
}
