use crate::{
    build,
    execution::{self, ExecutionArgs, ExecutionBackend},
    git, kubectl, tailscale,
};
use clap::{Args, ValueEnum};
use constcat::concat;
use home::home_dir;
use log::{debug, warn};

const DATABRICKSCFG_NAME: &str = "databrickscfg";
const DATABRICKSCFG_MOUNT: &str = "/root/.databrickscfg";

#[derive(Debug, Args)]
pub struct SubmitArgs {
    /// The minimum number of GPUs required to execute the work.
    #[arg(long = "gpus", default_value_t = 0)]
    pub gpus: u32,

    #[arg(long = "workers", default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub workers: u32,

    #[arg(long = "allow-dirty", default_value_t)]
    pub allow_dirty: bool,

    #[arg(long = "allow-unpushed", default_value_t)]
    pub allow_unpushed: bool,

    #[arg(long = "databrickscfg-mode", value_enum, default_value_t, help = concat!("Control whether a secret named \"", DATABRICKSCFG_NAME, "\" should be created from the submitting machine and mounted as a file at \"", DATABRICKSCFG_MOUNT, "\" through a volume in the container of the submitted job."))]
    pub databrickscfg_mode: DatabricksCfgMode,

    #[arg(required = true, last = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum DatabricksCfgMode {
    /// The databrickscfg secret will be created and attached to the container if possible.
    #[default]
    Auto,
    /// The databrickscfg secret is required.
    Require,
    /// The databrickscfg secret should be omitted.
    Omit,
}

use crate::Result;

fn get_user() -> Result<String> {
    let launched_by_user = tailscale::get_user()?;
    Ok(if launched_by_user.contains('@') {
        // The Tailscale login name refers to a person.
        launched_by_user
    } else {
        // The Tailscale login name refers to a machine, use the OS username instead.
        whoami::username()
    })
}

pub fn submit(args: SubmitArgs) -> Result<()> {
    let SubmitArgs {
        gpus,
        workers,
        allow_dirty,
        allow_unpushed,
        databrickscfg_mode,
        command,
    } = args;
    let image_registry_outside_cluster = "berkeley-docker.taila1eba.ts.net";
    // Configured in `k8s-cluster.yml` under `containerd_registries_mirrors`.
    let image_registry_inside_cluster = "astera-infra.com";
    let image_name = "fluid";
    let headlamp_base_url = "https://berkeley-headlamp.taila1eba.ts.net";

    if command.is_empty() {
        return Err("Please provide the command to run".into());
    }

    let launched_by_user = get_user()?;
    debug!("launched_by_user: {launched_by_user:?}");

    let launched_by_hostname = whoami::fallible::hostname()?;
    debug!("launched_by_hostname: {launched_by_hostname:?}");

    let git_info = git::info()?;

    if !allow_dirty && !git_info.is_clean {
        warn!("Please ensure that you commit all changes so we can reproduce the results. This warning may become an error in the future. You can disable this check by passing `--allow-dirty`.");
    }

    if !allow_unpushed && !git_info.is_pushed {
        warn!("Please ensure that your commit is pushed to a remote so we can reproduce the results. This warning may become an error in the future. You can disable this check by passing `--allow-unpushed`.");
    }

    let tag = format!("{image_registry_outside_cluster}/{image_name}:latest");
    let build_backend = &build::LocalBuildBackend as &dyn build::BuildBackend;
    let image_digest = build_backend
        .build(build::BuildArgs {
            git_commit_hash: &git_info.commit_hash,
            image_tag: &tag,
        })?
        .image_digest;
    debug!("image_digest: {image_digest:?}");

    let home_dir = home_dir().ok_or("failed to determine home directory")?;

    let kubectl = kubectl::berkeley();

    // Create databricks secret from file.
    let supply_databrickscfg = if matches!(
        databrickscfg_mode,
        DatabricksCfgMode::Auto | DatabricksCfgMode::Require
    ) {
        let databrickscfg_path = home_dir.join(".databrickscfg");
        if let Err(error) = std::fs::metadata(&databrickscfg_path) {
            if matches!(databrickscfg_mode, DatabricksCfgMode::Require) {
                return Err(format!(
                    "Databricks configuration not found at {databrickscfg_path:?}: {error}. \
                    Please follow the instructions at https://github.com/Astera-org/obelisk/blob/master/fluid/README.md#logging-to-mlflow."
                ).into());
            } else {
                warn!(
                    "Databricks configuration not found at {databrickscfg_path:?}: {error}. \
                    Please follow the instructions at https://github.com/Astera-org/obelisk/blob/master/fluid/README.md#logging-to-mlflow. \
                    To omit the databricks configuration and avoid this warning, pass `--databrickcfg-mode omit`."
                );
            }
            false
        } else {
            kubectl.recreate_secret_from_file(
                kubectl::LAUNCH_NAMESPACE,
                "databrickscfg",
                &databrickscfg_path,
            )?;
            true
        }
    } else {
        false
    };

    let (volume_mounts, volumes) = if supply_databrickscfg {
        (
            serde_json::json!([
                {
                    "name": DATABRICKSCFG_NAME,
                    "mountPath": DATABRICKSCFG_MOUNT,
                    "subPath": ".databrickscfg",
                    "readOnly": true
                }
            ]),
            serde_json::json!([
                {
                    "name": "databrickscfg",
                    "secret": {
                        "secretName": DATABRICKSCFG_NAME,
                    }
                }
            ]),
        )
    } else {
        (serde_json::json!([]), serde_json::json!([]))
    };

    let generate_name = format!(
        "launch-{}-",
        kubectl::to_rfc_1123_label_lossy(&launched_by_user).ok_or_else(|| format!(
            "Failed to generate job name from tailscale username {launched_by_user:?}"
        ))?
    );

    let execution_backend: &dyn ExecutionBackend = if workers > 1 {
        &execution::RayExecutionBackend
    } else {
        &execution::KubernetesExecutionBackend
    };

    execution_backend.execute(ExecutionArgs {
        kubectl: &kubectl,
        headlamp_base_url,
        job_namespace: kubectl::LAUNCH_NAMESPACE,
        generate_name: &generate_name,
        launched_by_user: &launched_by_user,
        launched_by_hostname: &launched_by_hostname,
        image_registry: image_registry_inside_cluster,
        image_name,
        image_digest: &image_digest,
        volume_mounts,
        volumes,
        command: &command,
        workers,
        gpus,
    })?;

    Ok(())
}
