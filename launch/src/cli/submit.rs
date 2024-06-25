use clap::{Args, ValueEnum};
use constcat::concat;
use home::home_dir;
use log::{debug, warn};

use super::ClusterContext;
use crate::{
    build,
    execution::{self, ExecutionArgs, ExecutionBackend},
    git,
    kubectl::{self, is_rfc_1123_label},
    unit::bytes::{self, Bytes},
    user_host::UserHost,
    Result,
};

fn gibibyte(s: &str) -> Result<Bytes> {
    Ok(Bytes::new::<bytes::gibibyte>(s.parse()?).ok_or_else(|| "value too large".to_string())?)
}

#[derive(Debug, Args)]
pub struct SubmitArgs {
    /// The minimum number of GPUs per worker.
    #[arg(long = "gpus", default_value_t)]
    pub gpus: u32,

    /// The minimum GPU RAM memory per worker in gibibyte (GiB, 2^30 bytes).
    #[arg(long = "gpu-mem", value_parser=gibibyte)]
    pub gpu_mem: Option<Bytes>,

    /// The number of workers to spawn. If the number of workers is larger than 1, the Ray execution backend will be
    /// used.
    #[arg(long = "workers", default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub workers: u32,

    #[arg(long = "allow-dirty", default_value_t)]
    pub allow_dirty: bool,

    #[arg(long = "allow-unpushed", default_value_t)]
    pub allow_unpushed: bool,

    /// Job name prefix of up to 20 characters, starting with an alphabetic character (a-z) and further consisting of
    /// alphanumeric characters (a-z, 0-9) optionally separated by dashes (-).
    #[arg(long = "name-prefix", value_parser = expect_name_prefix)]
    pub name_prefix: Option<String>,

    #[arg(long = "databrickscfg-mode", value_enum, default_value_t, help = concat!("Control whether a secret should be created from the submitting machine and mounted as a file at \"", execution::DATABRICKSCFG_MOUNT, "\" through a volume in the container of the submitted job."))]
    pub databrickscfg_mode: DatabricksCfgMode,

    #[arg(required = true, last = true)]
    pub command: Vec<String>,
}

fn expect_name_prefix(value: &str) -> Result<String, &'static str> {
    if !is_rfc_1123_label(value) {
        return Err("expected an RFC 1123 label matching regex /^[a-z]([a-z0-9]+-)*[a-z0-9]$/");
    }
    if value.len() > 20 {
        return Err("expected 20 characters or less");
    }
    Ok(value.to_string())
}

#[derive(Debug, Default, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DatabricksCfgMode {
    /// The databrickscfg secret will be created and attached to the container if possible.
    #[default]
    Auto,
    /// The databrickscfg secret is required.
    Require,
    /// The databrickscfg secret should be omitted.
    Omit,
}

pub fn submit(context: &ClusterContext, args: SubmitArgs) -> Result<()> {
    let SubmitArgs {
        gpus,
        gpu_mem,
        workers,
        allow_dirty,
        allow_unpushed,
        databrickscfg_mode,
        name_prefix,
        command,
    } = args;
    // Configured in `k8s-cluster.yml` under `containerd_registries_mirrors`.
    let image_name = "fluid";

    if command.is_empty() {
        return Err("Please provide the command to run".into());
    }

    let machine_user_host = super::common::machine_user_host();
    let tailscale_user_host = super::common::tailscale_user_host();
    let user = kubectl::to_rfc_1123_label_lossy(
        tailscale_user_host
            .as_ref()
            .and_then(|value| value.host().is_some().then_some(value.user()))
            .unwrap_or(machine_user_host.user()),
    );

    let git_info = git::info()?;

    if !allow_dirty && !git_info.is_clean {
        warn!("Please ensure that you commit all changes so we can reproduce the results. This warning may become an error in the future. You can disable this check by passing `--allow-dirty`.");
    }

    if !allow_unpushed && !git_info.is_pushed {
        warn!("Please ensure that your commit is pushed to a remote so we can reproduce the results. This warning may become an error in the future. You can disable this check by passing `--allow-unpushed`.");
    }

    let tag = format!("{host}/{image_name}:latest", host = context.docker_host());
    let build_backend = &build::LocalBuildBackend as &dyn build::BuildBackend;
    let image_digest = build_backend
        .build(build::BuildArgs {
            git_commit_hash: &git_info.commit_hash,
            image_tag: &tag,
        })?
        .image_digest;
    debug!("image_digest: {image_digest:?}");

    let home_dir = home_dir().ok_or("failed to determine home directory")?;

    let kubectl = context.kubectl();

    let databrickscfg_path = if matches!(
        databrickscfg_mode,
        DatabricksCfgMode::Auto | DatabricksCfgMode::Require
    ) {
        let path = home_dir.join(".databrickscfg");
        match std::fs::metadata(&path) {
            Ok(_) => Some(path),
            Err(error) => {
                let error_string = format!(
                    "Databricks configuration not found at {path:?}: {error}. \
                    Please follow the instructions at https://github.com/Astera-org/obelisk/blob/master/fluid/README.md#logging-to-mlflow."
                );
                if databrickscfg_mode == DatabricksCfgMode::Require {
                    return Err(error_string.into());
                } else {
                    warn!(
                        "{error_string} To omit the databricks configuration and avoid this warning, pass `--databrickcfg-mode omit`."
                    );
                    None
                }
            }
        }
    } else {
        None
    };

    let databrickscfg_name = databrickscfg_path
        .map(|path| -> Result<_> {
            let namespace = kubectl::NAMESPACE;
            let name = match user.as_deref() {
                Some(user) => format!("databrickscfg-{user}"),
                None => "databrickscfg".to_string(),
            };
            kubectl.recreate_secret_from_file(kubectl::NAMESPACE, &name, &path)?;
            debug!(
                "Created Secret {headlamp_url}/c/main/secrets/{namespace}/{name}",
                headlamp_url = context.headlamp_url()
            );
            Ok(name)
        })
        .transpose()?;

    enum ExecutionBackendKind {
        Job,
        RayJob,
    }

    let execution_backend_kind = if workers > 1 {
        ExecutionBackendKind::RayJob
    } else {
        ExecutionBackendKind::Job
    };

    let generate_name = {
        let mut name = String::new();

        if let Some(value) = name_prefix.as_deref() {
            name.push_str(value);
            name.push('-');
        };

        if let Some(user) = user.as_deref() {
            name.push_str(user);
            name.push('-')
        }

        if name.is_empty() {
            name.push_str(match execution_backend_kind {
                ExecutionBackendKind::Job => "job",
                ExecutionBackendKind::RayJob => "ray-job",
            });
            name.push('-');
        }

        name
    };

    let execution_backend: &dyn ExecutionBackend = match execution_backend_kind {
        ExecutionBackendKind::Job => &execution::KubernetesExecutionBackend,
        ExecutionBackendKind::RayJob => &execution::RayExecutionBackend,
    };

    execution_backend.execute(ExecutionArgs {
        context,
        job_namespace: kubectl::NAMESPACE,
        generate_name: &generate_name,
        machine_user_host: machine_user_host.to_ref(),
        tailscale_user_host: tailscale_user_host.as_ref().map(UserHost::to_ref),
        image_name,
        image_digest: &image_digest,
        databrickscfg_name: databrickscfg_name.as_deref(),
        command: &command,
        workers,
        gpus,
        gpu_mem,
    })?;

    Ok(())
}
