use std::path::PathBuf;

use clap::{Args, ValueEnum};
use constcat::concat;
use home::home_dir;
use log::{debug, warn};

use super::ClusterContext;
use crate::{
    builder,
    executor::{self, ExecutionArgs, Executor, ImageMetadata},
    git, katib,
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
    /// How to build the image.
    #[arg(long = "builder", value_enum, default_value_t)]
    pub builder: BuilderArg,

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

    /// Path to a Katib experiment spec YAML file.
    /// The valid fields are documented here, but note that trialTemplate is not allowed since
    /// the launch tool constructs that for you:
    /// https://www.kubeflow.org/docs/components/katib/user-guides/hp-tuning/configure-experiment/
    /// Any parameter listed in the config file will be passed as a command line arg to the given
    /// command. E.g. if ther is a parameter named "foo.bar", then each trial of the experiment
    /// will get "--foo.bar=<param value for that trial>" appended to the command.
    #[arg(long = "katib")]
    pub katib_path: Option<PathBuf>,

    #[arg(long = "databrickscfg-mode", value_enum, default_value_t, help = concat!("Control whether a secret should be created from the submitting machine and mounted as a file at \"", executor::DATABRICKSCFG_MOUNT, "\" through a volume in the container of the submitted job."))]
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
pub enum BuilderArg {
    /// Use `docker` to build the image locally.
    #[default]
    Docker,
    /// Use `kaniko` to build the image remotely.
    Kaniko,
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
        builder,
        gpus,
        gpu_mem,
        workers,
        allow_dirty,
        allow_unpushed,
        databrickscfg_mode,
        name_prefix,
        command,
        katib_path,
    } = args;

    if command.is_empty() {
        return Err("Please provide the command to run".into());
    }

    let current_dir = std::env::current_dir()?;
    let image_name = std::path::Path::new(&current_dir)
        .file_name()
        .ok_or("launch")?
        .to_str()
        .ok_or("Current directory name contains invalid UTF-8")?;

    let katib_experiment_spec = katib_path
        .as_ref()
        .map(|path| {
            std::fs::read_to_string(path)
                .map_err(|error| {
                    format!("Failed to read Katib experiment spec file {path:?}: {error}")
                })
                .and_then(|contents| {
                    serde_yaml::from_str::<katib::ExperimentSpec>(&contents).map_err(|err| {
                        format!(
                            "Failed to parse Katib experiment spec file {path:?}. See `launch submit --help` for format: {err}"
                        )
                    })
                })
        })
        .transpose()?;

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

    let image_name_with_tag = format!(
        "{host}/{image_name}:{user}-{rand:x}",
        host = context.container_registry_host(),
        user = user.as_deref().unwrap_or("unknown-user"),
        rand = rand::random::<u32>()
    );
    let build_backend = match builder {
        BuilderArg::Docker => &builder::DockerBuilder as &dyn builder::Builder,
        BuilderArg::Kaniko => &builder::KanikoBuilder as &dyn builder::Builder,
    };

    let build_output = build_backend.build(builder::BuildArgs {
        git_commit_hash: &git_info.commit_hash,
        image_name_with_tag: &image_name_with_tag,
    })?;

    let image_digest = build_output.image_digest;
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
                    Please follow the instructions at https://github.com/Astera-org/obelisk/blob/master/research/README.md#logging-to-mlflow."
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
        Katib,
        RayJob,
    }

    let execution_backend_kind = if katib_path.is_some() {
        ExecutionBackendKind::Katib
    } else if workers > 1 {
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
                ExecutionBackendKind::Katib => "katib",
                ExecutionBackendKind::RayJob => "ray-job",
            });
            name.push('-');
        }

        name
    };

    let execution_backend: &dyn Executor = match execution_backend_kind {
        ExecutionBackendKind::Job => &executor::KubernetesExecutionBackend,
        ExecutionBackendKind::Katib => &executor::KatibExecutionBackend,
        ExecutionBackendKind::RayJob => &executor::RayExecutionBackend,
    };

    let image_metadata = ImageMetadata {
        digest: image_digest.as_ref(),
        name: image_name,
    };

    execution_backend.execute(ExecutionArgs {
        context,
        job_namespace: kubectl::NAMESPACE,
        generate_name: &generate_name,
        machine_user_host: machine_user_host.to_ref(),
        tailscale_user_host: tailscale_user_host.as_ref().map(UserHost::to_ref),
        image_metadata,
        databrickscfg_name: databrickscfg_name.as_deref(),
        container_args: &command,
        workers,
        gpus,
        gpu_mem,
        katib_experiment_spec,
    })?;

    Ok(())
}
