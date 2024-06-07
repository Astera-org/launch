pub(crate) mod common;
mod kubernetes;
mod ray;

pub use kubernetes::*;
pub use ray::*;

use crate::kubectl::Kubectl;

pub struct ExecutionArgs<'a> {
    pub kubectl: &'a Kubectl,
    pub headlamp_base_url: &'a str,
    pub job_namespace: &'a str,
    pub generate_name: &'a str,
    pub launched_by_user: &'a str,
    pub launched_by_hostname: &'a str,
    pub image_registry: &'a str,
    pub image_name: &'a str,
    pub image_digest: &'a str,
    pub volume_mounts: serde_json::Value,
    pub volumes: serde_json::Value,
    pub command: &'a [String],
    pub workers: u32,
    pub gpus: u32,
}

impl<'a> ExecutionArgs<'a> {
    pub fn image(&self) -> String {
        format!(
            "{registry}/{name}@{digest}",
            registry = self.image_registry,
            name = self.image_name,
            digest = self.image_digest
        )
    }
}

pub struct ExecutionOutput {}

pub trait ExecutionBackend {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput>;
}

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
