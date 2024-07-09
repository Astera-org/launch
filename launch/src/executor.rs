mod common;
mod kubernetes;
mod ray;

pub use kubernetes::*;
pub use ray::*;

use crate::{
    cli::ClusterContext,
    kubectl::{self},
    unit::bytes::{self, Bytes},
    user_host::UserHostRef,
    Result,
};

pub struct ExecutionArgs<'a> {
    pub context: &'a ClusterContext,
    pub job_namespace: &'a str,
    pub generate_name: &'a str,
    pub machine_user_host: UserHostRef<'a>,
    pub tailscale_user_host: Option<UserHostRef<'a>>,
    pub image_name: &'a str,
    pub image_digest: &'a str,
    pub databrickscfg_name: Option<&'a str>,
    pub command: &'a [String],
    pub workers: u32,
    pub gpus: u32,
    pub gpu_mem: Option<Bytes>,
}

pub const DATABRICKSCFG_MOUNT: &str = "/root/.databrickscfg";

impl<'a> ExecutionArgs<'a> {
    fn image(&self) -> String {
        format!(
            "{host}/{name}@{digest}",
            host = self.context.docker_host_inside_cluster(),
            name = self.image_name,
            digest = self.image_digest
        )
    }

    fn annotations(&self) -> impl serde::Serialize {
        use std::borrow::Cow;

        use kubectl::annotation;

        [
            (annotation::VERSION, Cow::Borrowed(crate::version::VERSION)),
            (
                annotation::LAUNCHED_BY_MACHINE_USER,
                Cow::Owned(self.machine_user_host.to_string()),
            ),
        ]
        .into_iter()
        .chain(self.tailscale_user_host.as_ref().map(|value| {
            (
                annotation::LAUNCHED_BY_TAILSCALE_USER,
                Cow::Owned(value.to_string()),
            )
        }))
        .collect::<std::collections::HashMap<_, _>>()
    }

    fn volume_mounts(&self) -> impl serde::Serialize {
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

    fn volumes(&self) -> impl serde::Serialize {
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

    fn resources(&self) -> impl serde::Serialize {
        if self.gpus != 0 {
            serde_json::json!({
                "limits": {
                    "nvidia.com/gpu": self.gpus
                }
            })
        } else {
            serde_json::json!(null)
        }
    }

    fn affinity(&self) -> impl serde::Serialize {
        let gpu_mem_mib = self
            .gpu_mem
            .map(|gpu_mem| gpu_mem.get::<bytes::mebibyte>())
            .unwrap_or_default();
        if gpu_mem_mib != 0 {
            serde_json::json!({
                "nodeAffinity": {
                    "requiredDuringSchedulingIgnoredDuringExecution": {
                        "nodeSelectorTerms": [
                            {
                                "matchExpressions": [
                                    {
                                        "key": "nvidia.com/gpu.memory",
                                        // Sub 1 so that a user's request for `>= X` becomes `> (X - 1)`.
                                        "operator": "Gt",
                                        // `values` only accepts strings so integers must be converted to strings.
                                        "values": [
                                            gpu_mem_mib.saturating_sub(1).to_string()
                                        ]
                                    }
                                ]
                            }
                        ]
                    }
                }
            })
        } else {
            serde_json::json!(null)
        }
    }

    fn env(&self) -> impl serde::Serialize {
        #[derive(serde::Serialize)]
        struct SetEnv<'a> {
            name: &'a str,
            value: &'a str,
        }

        impl<'a> SetEnv<'a> {
            pub fn new(name: &'a str, value: &'a str) -> Self {
                Self { name, value }
            }
        }

        // Suppress warnings from GitPython (used by mlflow) about the git executable not being available.
        [SetEnv::new("GIT_PYTHON_REFRESH", "QUIET")]
    }
}

pub struct ExecutionOutput {}

pub trait Executor {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput>;
}
