use std::path::Path;

use crate::{as_ref, process};

mod pod_status;
pub use pod_status::*;

mod name;
pub use name::*;

pub struct Kubectl {
    server: String,
}

impl Kubectl {
    pub fn new(server: String) -> Self {
        Self { server }
    }

    /// Returns the kubectl command where authentication arguments have already been set.
    fn kubectl(&self) -> process::Command {
        process::Command::new("kubectl").args(as_ref![
            // Despite passing `--server` and `--token`, kubectl will still load the kubeconfig if
            // present. By setting `--kubeconfig` to an empty file, we can make sure no other
            // options apply.
            "--kubeconfig=/dev/null", // Does not work on Windows but Windows users develop inside WSL.
            "--server",
            self.server,
            "--token=unused",
        ])
    }

    pub fn recreate_secret_from_file(
        &self,
        namespace: &str,
        name: &str,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.kubectl()
            .args(as_ref![
                "delete",
                "secret",
                "--ignore-not-found",
                "--namespace",
                namespace,
                name,
            ])
            .status()?;

        self.kubectl()
            .args(as_ref![
                "create",
                "secret",
                "generic",
                "--from-file",
                path,
                "--namespace",
                namespace,
                name,
            ])
            .status()?;

        Ok(())
    }

    /// The input is written to stdin and should be a [YAML or JSON formatted kubernetes
    /// configuration](https://kubernetes.io/docs/tasks/manage-kubernetes-objects/imperative-config/).
    pub fn create_job(&self, input: &str) -> Result<JobHandle, Box<dyn std::error::Error>> {
        let output = self
            .kubectl()
            .args(as_ref!["create", "--output=json", "-f", "-"])
            .output_with_input(input.as_bytes().to_owned())?;

        let root: CreateJobRoot = serde_json::from_slice(&output.stdout)?;

        Ok(JobHandle {
            namespace: root.metadata.namespace,
            job_name: root.metadata.name,
        })
    }

    pub fn get_pods_for_job(
        &self,
        namespace: &str,
        job_name: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let output = self
            .kubectl()
            .args(as_ref![
                "get",
                "pods",
                "--namespace",
                namespace,
                format!("--selector=job-name={job_name}"),
                "--output=jsonpath={.items[*].metadata.name}"
            ])
            .output()?;

        Ok(std::str::from_utf8(&output.stdout)?
            .split_whitespace()
            .map(str::to_string)
            .collect())
    }

    pub fn follow_pod_logs(
        &self,
        namespace: &str,
        pod_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.kubectl()
            .args(as_ref!["logs", "--namespace", namespace, "-f", pod_name,])
            .status()?;
        Ok(())
    }

    pub fn describe_pod(
        &self,
        namespace: &str,
        pod_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.kubectl()
            .args(as_ref![
                "describe",
                "pod",
                "--namespace",
                namespace,
                pod_name,
            ])
            .status()?;
        Ok(())
    }

    pub fn pod_status(
        &self,
        namespace: &str,
        pod_name: &str,
    ) -> Result<PodStatus, Box<dyn std::error::Error>> {
        let output = self
            .kubectl()
            .args(as_ref![
                "get",
                "pod",
                "--namespace",
                namespace,
                pod_name,
                "--output=json",
            ])
            .output()?;

        let root: PodStatusRoot = serde_json::from_slice(&output.stdout)?;

        Ok(root.status)
    }
}

#[derive(Debug)]
pub struct JobHandle {
    pub namespace: String,
    pub job_name: String,
}

#[derive(serde::Deserialize)]
struct CreateJobRoot {
    metadata: CreateOutputMetadata,
}

#[derive(serde::Deserialize)]
struct CreateOutputMetadata {
    name: String,
    namespace: String,
}

// https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-phase
#[derive(Debug, serde::Deserialize)]
pub enum PodStatusPhase {
    /// The pod has been accepted by the Kubernetes system, but one or more of the containers have
    /// not been started. This includes time spent waiting for the scheduler to schedule the pod and
    /// for the pod to be downloaded and for images to be downloaded.
    #[serde(rename = "Pending")]
    Pending,

    /// The pod has been bound to a node, and all of the containers have been started. At least one
    /// container is still running, or is in the process of starting or restarting.
    #[serde(rename = "Running")]
    Running,

    /// All containers in the pod have terminated successfully, and they will not be restarted.
    #[serde(rename = "Succeeded")]
    Succeeded,

    /// All containers in the pod have terminated, and at least one container has terminated in
    /// failure. That is, the container either exited with a non-zero status or was terminated by
    /// the system.
    #[serde(rename = "Failed")]
    Failed,

    /// The state of the pod could not be obtained, typically due to an error in communicating with
    /// the host of the pod.
    #[serde(rename = "Unknown")]
    Unknown,
}

// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#podstatus-v1-core
#[derive(Debug, serde::Deserialize)]
pub struct PodStatusRoot {
    pub status: PodStatus,
}
