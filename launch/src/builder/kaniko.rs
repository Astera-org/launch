use std::path::Path;

use ::kubernetes::models as k8s;
use log::debug;

use super::{BuildArgs, BuildOutput, Builder, Result};
use crate::{
    executor::{self, Deadline, KANIKO_POST_BUILD_TIMEOUT, POLLING_INTERVAL},
    kubectl::{self},
};

// see ansible/playbooks/roles/talos_k8s_configs/templates/launch.yml
pub const KANIKO_GITHUB_TOKEN: &str = "kaniko-github-token";
pub const KANIKO_CACHE_PVC_NAME: &str = "kaniko-cache";
pub const KANIKO_CACHE_PVC_MOUNT_PATH: &str = "/var/run/uv";

pub struct KanikoBuilder<'a> {
    pub kubectl: &'a kubectl::Kubectl<'a>,
    pub namespace: &'a str,
    pub user: Option<&'a str>,
    pub working_directory: &'a Path,
}

impl Builder for KanikoBuilder<'_> {
    fn build<'a>(&'a self, args: BuildArgs<'a>) -> Result<BuildOutput> {
        let Self { kubectl, .. } = self;
        debug!("Building image: {:?}", args.image);

        let pod = kubectl.create(&serde_json::to_string(&self.pod_spec(&args)?)?)?;

        executor::wait_for_and_follow_pod_logs(kubectl, &pod.namespace, &pod.name)?;

        // Pod status has a lag to update, so we need to wait
        let deadline = Deadline::after(KANIKO_POST_BUILD_TIMEOUT);
        let status = loop {
            let status = kubectl.pod(&pod.namespace, &pod.name)?.status;
            debug!("Pod status: {status}");

            match &status.phase {
                kubectl::PodPhase::Running => {
                    deadline.sleep(POLLING_INTERVAL).map_err(|_| {
                        "deadline exceeded while waiting for kaniko build pod to finish"
                    })?;
                }
                kubectl::PodPhase::Succeeded => {
                    break status;
                }
                kubectl::PodPhase::Failed => {
                    return Err("kaniko build failed, inspect the build output to learn why".into())
                }
                other => return Err(format!("unespected status {}", other).into()),
            }
        };

        // We control the pod spec, there should be only a single container status.
        let container_status = {
            let mut iter = status.container_statuses.into_iter();
            let Some(first) = iter.next() else {
                return Err("pod does not have container statuses".into());
            };
            let None = iter.next() else {
                return Err("pod has more than one container statuses".into());
            };
            first
        };

        let state = match container_status.state {
            kubectl::ContainerState::Terminated(state) => state,
            other => return Err(format!("unexpected termination state: {}", other).into()),
        };

        let digest = state
            .message
            .as_deref()
            .ok_or("build container should have termination state message")?
            .trim();

        Ok(BuildOutput {
            digest: digest.to_string(),
        })
    }
}

impl KanikoBuilder<'_> {
    fn pod_spec(&self, args: &BuildArgs) -> Result<k8s::V1Pod> {
        let Self {
            working_directory,
            namespace,
            user,
            ..
        } = *self;

        let generate_name = {
            let mut out = "kaniko-".to_owned();
            if let Some(user) = user {
                out.push_str(user);
                out.push('-');
            }
            out
        };

        // TODO support repo git url
        let push_remote = "github.com/Astera-org/obelisk";

        // Does not take into account symlinks and what not, should be good enough.
        let sub_path = working_directory
            .strip_prefix(&args.git_info.dir)?
            .to_owned();

        // Prefer Dockerfile.kaniko if it exists
        let mut dockerfile = "Dockerfile";
        if working_directory.join("Dockerfile.kaniko").exists() {
            dockerfile = "Dockerfile.kaniko";
        }

        Ok(k8s::V1Pod {
            api_version: Some("v1".to_owned()),
            kind: Some("Pod".to_owned()),
            metadata: Some(Box::new(k8s::V1ObjectMeta {
                namespace: Some(namespace.to_string()),
                generate_name: Some(generate_name.to_owned()),
                ..Default::default()
            })),
            spec: Some(Box::new(k8s::V1PodSpec {
                restart_policy: Some("Never".to_owned()),
                containers: vec![k8s::V1Container {
                    name: "main".to_owned(),
                    image: Some("gcr.io/kaniko-project/executor:latest".to_owned()),
                    args: Some(vec![
                        format!(
                            "--context=git://{push_remote}#{commit}",
                            commit = args.git_info.commit_hash
                        ),
                        format!("--context-sub-path={}", sub_path.display()),
                        // explicitly specify dockerfile, to support kaniko Dockerfile
                        format!("--dockerfile={}", dockerfile),
                        format!("--destination={}", args.image),
                        format!("--build-arg=COMMIT_HASH={}", args.git_info.commit_hash),
                        // allow push to cluster registry
                        "--insecure".to_owned(),
                        // allow push without auth
                        "--skip-push-permission-check".to_owned(),
                        // perf: only clone the current branch
                        "--git=single-branch=true".to_owned(),
                        // Write the digest to the default kubernetes termination log. See https://github.com/GoogleContainerTools/kaniko/blob/main/README.md#flag---digest-file
                        "--digest-file=/dev/termination-log".to_owned(),
                    ]),
                    env_from: Some(vec![k8s::V1EnvFromSource {
                        secret_ref: Some(Box::new(k8s::V1SecretEnvSource {
                            name: Some(KANIKO_GITHUB_TOKEN.to_owned()),
                            optional: None,
                        })),
                        ..Default::default()
                    }]),
                    volume_mounts: Some(vec![k8s::V1VolumeMount {
                        name: KANIKO_CACHE_PVC_NAME.to_owned(),
                        mount_path: KANIKO_CACHE_PVC_MOUNT_PATH.to_owned(),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }],
                volumes: Some(vec![k8s::V1Volume {
                    name: KANIKO_CACHE_PVC_NAME.to_owned(),
                    persistent_volume_claim: Some(Box::new(
                        k8s::V1PersistentVolumeClaimVolumeSource {
                            claim_name: KANIKO_CACHE_PVC_NAME.to_owned(),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                }]),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
