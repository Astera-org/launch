pub(crate) mod common;
mod kubernetes;
mod ray;

use std::collections::HashMap;

pub use kubernetes::*;
pub use ray::*;

use crate::{
    kubectl::{self, Kubectl},
    user_host::UserHostRef,
};

pub struct ExecutionArgs<'a> {
    pub kubectl: &'a Kubectl,
    pub headlamp_base_url: &'a str,
    pub job_namespace: &'a str,
    pub generate_name: &'a str,
    pub machine_user_host: UserHostRef<'a>,
    pub tailscale_user_host: Option<UserHostRef<'a>>,
    pub image_registry: &'a str,
    pub image_name: &'a str,
    pub image_digest: &'a str,
    pub databrickscfg_name: Option<&'a str>,
    pub command: &'a [String],
    pub workers: u32,
    pub gpus: u32,
}

pub const DATABRICKSCFG_MOUNT: &str = "/root/.databrickscfg";

impl<'a> ExecutionArgs<'a> {
    fn image(&self) -> String {
        format!(
            "{registry}/{name}@{digest}",
            registry = self.image_registry,
            name = self.image_name,
            digest = self.image_digest
        )
    }

    fn annotations(&self) -> HashMap<&str, String> {
        let mut map = HashMap::new();
        map.insert(
            kubectl::annotation::LAUNCHED_BY_MACHINE_USER,
            self.machine_user_host.to_string(),
        );
        if let Some(ref value) = self.tailscale_user_host {
            map.insert(
                kubectl::annotation::LAUNCHED_BY_TAILSCALE_USER,
                value.to_string(),
            );
        }
        map
    }

    fn volume_mounts(&self) -> serde_json::Value {
        if self.databrickscfg_name.is_some() {
            serde_json::json!([
                {
                    "name": "databrickscfg",
                    "mountPath": DATABRICKSCFG_MOUNT,
                    "subPath": ".databrickscfg",
                    "readOnly": true
                }
            ])
        } else {
            serde_json::json!([])
        }
    }

    fn volumes(&self) -> serde_json::Value {
        if let Some(name) = self.databrickscfg_name {
            serde_json::json!([
                {
                    "name": "databrickscfg",
                    "secret": {
                        "secretName": name,
                    }
                }
            ])
        } else {
            serde_json::json!([])
        }
    }
}

pub struct ExecutionOutput {}

pub trait ExecutionBackend {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput>;
}

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
