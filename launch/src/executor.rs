mod katib;
mod kubernetes;
mod ray;

pub(crate) mod common;
use std::collections::HashMap;

use ::kubernetes::models as km;
pub use common::*;
use container_image_name::ImageNameRef;
pub use katib::*;
pub use kubernetes::*;
pub use ray::*;

use crate::{
    cli::ClusterContext,
    katib::ExperimentSpec,
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
    pub image: ImageNameRef<'a>,
    pub databrickscfg_name: Option<&'a str>,
    pub container_args: &'a [String],
    pub workers: u32,
    pub gpus: u32,
    pub gpu_mem: Option<Bytes>,
    pub katib_experiment_spec: Option<ExperimentSpec>,
}

pub const DATABRICKSCFG_MOUNT: &str = "/root/.databrickscfg";

impl ExecutionArgs<'_> {
    fn annotations(&self) -> HashMap<String, String> {
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
        .map(|(a, b)| (a.to_owned(), b.into_owned()))
        .collect::<std::collections::HashMap<_, _>>()
    }

    fn volume_mounts(&self) -> Option<Vec<km::V1VolumeMount>> {
        if self.databrickscfg_name.is_some() {
            Some(vec![km::V1VolumeMount {
                name: "databrickscfg".to_owned(),
                mount_path: DATABRICKSCFG_MOUNT.to_owned(),
                sub_path: Some(".databrickscfg".to_owned()),
                read_only: Some(true),
                ..Default::default()
            }])
        } else {
            None
        }
    }

    fn volumes(&self) -> Option<Vec<km::V1Volume>> {
        self.databrickscfg_name.map(|name| {
            vec![km::V1Volume {
                name: "databrickscfg".to_owned(),
                secret: Some(Box::new(km::V1SecretVolumeSource {
                    secret_name: Some(name.to_owned()),
                    ..Default::default()
                })),
                ..Default::default()
            }]
        })
    }

    fn resources(&self) -> Option<km::V1ResourceRequirements> {
        if self.gpus != 0 {
            Some(km::V1ResourceRequirements {
                limits: Some(
                    [("nvidia.com/gpu".to_owned(), self.gpus.to_string())]
                        .into_iter()
                        .collect(),
                ),
                ..Default::default()
            })
        } else {
            None
        }
    }

    fn affinity(&self) -> Option<km::V1Affinity> {
        let gpu_mem_mib = self
            .gpu_mem
            .map(|gpu_mem| gpu_mem.get::<bytes::mebibyte>())
            .unwrap_or_default();
        if gpu_mem_mib != 0 {
            Some(km::V1Affinity {
                node_affinity: Some(Box::new(km::V1NodeAffinity {
                    required_during_scheduling_ignored_during_execution: Some(Box::new(
                        km::V1NodeSelector {
                            node_selector_terms: vec![km::V1NodeSelectorTerm {
                                match_expressions: Some(vec![km::V1NodeSelectorRequirement {
                                    key: "nvidia.com/gpu.memory".to_string(),
                                    operator: "Gt".to_string(),
                                    // Sub 1 so that a user's request for `>= X` becomes `> (X - 1)`.
                                    values: Some(vec![gpu_mem_mib.saturating_sub(1).to_string()]),
                                }]),
                                ..Default::default()
                            }],
                        },
                    )),
                    ..Default::default()
                })),
                ..Default::default()
            })
        } else {
            None
        }
    }

    fn env(&self) -> Option<Vec<km::V1EnvVar>> {
        Some(vec![
            // Suppress warnings from GitPython (used by mlflow)
            // about the git executable not being available.
            km::V1EnvVar {
                name: "GIT_PYTHON_REFRESH".to_owned(),
                value: Some("quiet".to_owned()),
                ..Default::default()
            },
        ])
    }
}

pub struct ExecutionOutput {}

pub trait Executor {
    fn execute(&self, args: ExecutionArgs) -> Result<ExecutionOutput>;
}
