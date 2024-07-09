use serde::Deserialize;

use super::ResourceMetadata;

#[derive(Debug, Deserialize)]
/// https://github.com/ray-project/kuberay/blob/master/docs/reference/api.md#rayjob
pub struct RayJob {
    pub metadata: ResourceMetadata,
    pub status: RayJobStatus,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RayJobStatus {
    #[serde(rename = "jobId")]
    pub job_id: String,

    #[serde(rename = "jobStatus", default)]
    pub job_status: Option<String>,

    #[serde(rename = "jobDeploymentStatus")]
    pub job_deployment_status: String,

    #[serde(rename = "startTime", default, with = "time::serde::rfc3339::option")]
    pub start_time: Option<time::OffsetDateTime>,

    #[serde(rename = "endTime", default, with = "time::serde::rfc3339::option")]
    pub end_time: Option<time::OffsetDateTime>,

    #[serde(rename = "rayClusterName", default)]
    pub ray_cluster_name: Option<String>,

    #[serde(rename = "rayClusterStatus")]
    pub ray_cluster_status: RayJobStatusRayClusterStatus,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RayJobStatusRayClusterStatus {
    #[serde(rename = "state", default)]
    pub state: Option<String>,

    #[serde(
        rename = "lastUpdateTime",
        default,
        with = "time::serde::rfc3339::option"
    )]
    pub last_update_time: Option<time::OffsetDateTime>,
}
