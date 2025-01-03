use std::path::Path;

use crate::{process, Result};

mod node;
pub use node::*;

mod pod;
pub use pod::*;

mod name;
pub use name::*;

mod ray_job;
pub use ray_job::*;

mod job;
pub use job::*;

mod common;
pub use common::*;

pub struct Kubectl<'a> {
    server: &'a str,
}

impl<'a> Kubectl<'a> {
    pub fn new(server: &'a str) -> Self {
        Self { server }
    }

    /// Returns the kubectl command where authentication arguments have already been set.
    fn kubectl(&self) -> process::Command {
        process::command!(
            "kubectl",
            // Despite passing `--server` and `--token`, kubectl will still load the kubeconfig if
            // present. By setting `--kubeconfig` to an empty file, we can make sure no other
            // options apply.
            "--kubeconfig=/dev/null", // Does not work on Windows but Windows users develop inside WSL.
            "--server",
            self.server,
            "--token=unused",
        )
    }

    pub fn recreate_secret_from_file(
        &self,
        namespace: &str,
        name: &str,
        path: &Path,
    ) -> Result<()> {
        process::args!(
            self.kubectl(),
            "delete",
            "secret",
            "--ignore-not-found",
            "--namespace",
            namespace,
            name,
        )
        .output()?
        .require_success()?;

        process::args!(
            self.kubectl(),
            "create",
            "secret",
            "generic",
            "--from-file",
            path,
            "--namespace",
            namespace,
            name,
        )
        .output()?
        .require_success()?;

        Ok(())
    }

    pub fn nodes(&self) -> Result<Vec<Node>> {
        let output = process::args!(self.kubectl(), "get", "nodes", "--output=json").output()?;

        Ok(serde_json::from_slice::<GetResource<_>>(&output.stdout)?.items)
    }

    /// The input is written to stdin and should be a [YAML or JSON formatted kubernetes
    /// configuration](https://kubernetes.io/docs/tasks/manage-kubernetes-objects/imperative-config/).
    pub fn create(&self, input: &str) -> Result<ResourceHandle> {
        let output = process::args!(self.kubectl(), "create", "--output=json", "-f", "-")
            .output_with_input(input.as_bytes().to_owned())?;

        // The following should probably be integrated with a custom error type, but useful and good enough for now.
        if log::log_enabled!(log::Level::Error) && !output.status.success() {
            if let Ok(stderr) = std::str::from_utf8(&output.stderr) {
                let path = crate::temp_path::tmp_json_path();
                if std::fs::write(&path, input).is_ok() {
                    log::error!("Invalid spec (written to {}): {stderr}", path.display())
                }
            }
        }

        let output = output.require_success()?;

        let root: CreateJobRoot = serde_json::from_slice(&output.stdout)?;

        Ok(ResourceHandle {
            namespace: root.metadata.namespace,
            name: root.metadata.name,
        })
    }

    pub fn try_get_job(&self, namespace: &str, job_name: &str) -> Result<Option<Job>> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "job",
            "--namespace",
            namespace,
            job_name,
            "--output=json"
        )
        .try_output()?;

        let process::Output { command, output } = output;

        if output.status.success() {
            Ok(Some(serde_json::from_slice(&output.stdout)?))
        } else if output.stderr.starts_with(b"Error from server (NotFound): ") {
            Ok(None)
        } else {
            Err(process::Error {
                command,
                kind: process::ErrorKind::NonZeroExitStatus(
                    output.status.code().and_then(std::num::NonZeroI32::new),
                ),
            }
            .into())
        }
    }

    pub fn pods(&self, namespace: &str) -> Result<Vec<Pod>> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "pods",
            "--namespace",
            namespace,
            "--output=json"
        )
        .output()?;

        Ok(serde_json::from_slice::<GetResource<_>>(&output.stdout)?.items)
    }

    pub fn get_pods_for_job(&self, namespace: &str, job_name: &str) -> Result<Vec<String>> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "pods",
            "--namespace",
            namespace,
            format!("--selector=job-name={job_name}"),
            "--output=jsonpath={.items[*].metadata.name}"
        )
        .output()?;

        Ok(std::str::from_utf8(&output.stdout)?
            .split_whitespace()
            .map(str::to_string)
            .collect())
    }

    pub fn follow_pod_logs(&self, namespace: &str, pod_name: &str) -> Result<()> {
        process::args!(
            self.kubectl(),
            "logs",
            "--namespace",
            namespace,
            "-f",
            pod_name
        )
        .status()?;
        Ok(())
    }

    pub fn pod(&self, namespace: &str, pod_name: &str) -> Result<Pod> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "pod",
            "--namespace",
            namespace,
            pod_name,
            "--output=json",
        )
        .output()?;

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    pub fn jobs(&self, namespace: &str) -> Result<Vec<Job>> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "jobs",
            "--namespace",
            namespace,
            "--output=json"
        )
        .output()?;

        Ok(serde_json::from_slice::<GetResource<_>>(&output.stdout)?.items)
    }

    pub fn katib_experiment(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<::katib::models::V1beta1Experiment> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "experiment",
            "--namespace",
            namespace,
            name,
            "--output=json",
        )
        .output()?;

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    pub fn ray_jobs(&self, namespace: &str) -> Result<Vec<RayJob>> {
        let output = process::args!(
            self.kubectl(),
            "get",
            "rayjobs",
            "--namespace",
            namespace,
            "--output=json"
        )
        .output()?;

        Ok(serde_json::from_slice::<GetResource<_>>(&output.stdout)?.items)
    }

    pub fn delete_job(&self, job_name: &str, namespace: &str) -> Result<()> {
        let _ = process::args!(
            self.kubectl(),
            "--namespace",
            namespace,
            "delete",
            "job",
            job_name
        )
        .output()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ResourceHandle {
    pub namespace: String,
    pub name: String,
}

impl From<CreateJobRoot> for ResourceHandle {
    fn from(value: CreateJobRoot) -> Self {
        let CreateOutputMetadata { namespace, name } = value.metadata;
        Self { namespace, name }
    }
}
#[derive(serde::Deserialize)]
struct CreateJobRoot {
    metadata: CreateOutputMetadata,
}

#[derive(serde::Deserialize)]
struct CreateOutputMetadata {
    namespace: String,
    name: String,
}

pub const NAMESPACE: &str = "launch";

pub mod annotation {
    pub const LAUNCHED_BY_MACHINE_USER: &str = "launch.astera.org/launched-by-machine-user";
    pub const LAUNCHED_BY_TAILSCALE_USER: &str = "launch.astera.org/launched-by-tailscale-user";
    pub const VERSION: &str = "launch.astera.org/version";
}
