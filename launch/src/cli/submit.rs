use std::path::PathBuf;

use clap::{Args, ValueEnum};
use constcat::concat;
use container_image_name::ImageName;
use home::home_dir;
use log::{debug, warn};

use super::ClusterContext;
use crate::{
    builder,
    executor::{self, ExecutionArgs, Executor as _},
    git,
    kubectl::{self, is_rfc_1123_label, NAMESPACE},
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

    let machine_user_host = super::common::machine_user_host();
    let tailscale_user_host = super::common::tailscale_user_host();
    let user = kubectl::to_rfc_1123_label_lossy(
        tailscale_user_host
            .as_ref()
            .and_then(|value| value.host().is_some().then_some(value.user()))
            .unwrap_or(machine_user_host.user()),
    );

    let kubectl = context.kubectl();
    let git_info = git::info()?;

    if !allow_dirty && !git_info.is_clean {
        match builder {
            BuilderArg::Docker => warn!("Please ensure that you commit all changes so we can reproduce the results. This warning may become an error in the future. You can disable this check by passing `--allow-dirty`."),
            BuilderArg::Kaniko => return Err("There are git changes that have not been committed and pushed. When using the kaniko builder, this means the launched job will not have your latest code. Either commit and push all changes, or disable this check by passing `--allow-dirty`.".into()),
        }
    }

    if !allow_unpushed && !git_info.is_pushed {
        match builder {
            BuilderArg::Docker => warn!("Please ensure that your commit is pushed so we can reproduce the results. This warning may become an error in the future. You can disable this check by passing `--allow-unpushed`."),
            BuilderArg::Kaniko => return Err("There are git changes that have not been pushed. When using the kaniko builder, this means the launched job will not have your latest code. Either push all changes, or disable this check by passing `--allow-dirty`.".into()),
        }
    }

    let build_backend = match builder {
        BuilderArg::Docker => &builder::DockerBuilder as &dyn builder::Builder,
        BuilderArg::Kaniko => &builder::KanikoBuilder {
            working_directory: &std::env::current_dir()?,
            kubectl: &kubectl,
            namespace: NAMESPACE,
            user: user.as_deref(),
        } as &dyn builder::Builder,
    };

    let tagged_image = {
        let current_dir = std::env::current_dir()?;

        let image_name = std::path::Path::new(&current_dir)
            .file_name()
            .ok_or("launch")?
            .to_str()
            .ok_or("Current directory name contains invalid UTF-8")?;

        let image_tag = format!(
            "{user}-{rand:x}",
            user = user.as_deref().unwrap_or("unknown-user"),
            rand = rand::random::<u32>()
        );

        // Kaniko should directly push to the cluster local registry, and not the Tailscale registry
        // proxy, for performance
        let image_registry = match builder {
            BuilderArg::Docker => context.container_registry_host(),
            BuilderArg::Kaniko => "docker-registry.docker-registry.svc.cluster.local",
        };

        ImageName::builder(image_name)
            .with_registry(image_registry)
            .with_tag(image_tag)
            .build()?
    };

    let build_output = build_backend.build(builder::BuildArgs {
        git_info: &git_info,
        image: tagged_image.as_ref(),
    })?;

    let built_image = tagged_image
        .as_builder()
        .with_registry(context.container_registry_host())
        .with_digest(&build_output.digest)
        .build()
        .map_err(|_| {
            format!(
                "failed to combine image {:?} with digest {:?}",
                tagged_image, build_output.digest
            )
        })
        .unwrap();

    debug!("Built container image: {}", built_image);
    let home_dir = home_dir().ok_or("failed to determine home directory")?;

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

    let executor: executor::AnyExecutor = if let Some(experiment_spec_path) = katib_path {
        if workers > 1 {
            // TODO: Consider refactoring the argument parsing to prohibit this.
            warn!("The katib execution backend ignores the workers argument. Configure `parallelTrialCount` in the experiment specification instead.")
        }
        executor::KatibExecutor {
            experiment_spec_path,
        }
        .into()
    } else if workers > 1 {
        executor::RayExecutor.into()
    } else {
        executor::KubernetesExecutor.into()
    };

    let generate_name = generate_name(name_prefix.as_deref(), user.as_deref(), &executor);

    executor.execute(ExecutionArgs {
        context,
        job_namespace: kubectl::NAMESPACE,
        generate_name: &generate_name,
        machine_user_host: machine_user_host.to_ref(),
        tailscale_user_host: tailscale_user_host.as_ref().map(UserHost::to_ref),
        image: built_image.as_ref(),
        databrickscfg_name: databrickscfg_name.as_deref(),
        container_args: &command,
        workers,
        gpus,
        gpu_mem,
    })?;

    Ok(())
}

fn generate_name(
    name_prefix: Option<&str>,
    user: Option<&str>,
    executor: &executor::AnyExecutor,
) -> String {
    let mut name = String::new();

    if let Some(value) = name_prefix {
        name.push_str(value);
        name.push('-');
    };

    if let Some(user) = user {
        name.push_str(user);
        name.push('-')
    }

    if name.is_empty() {
        name.push_str(match *executor {
            executor::AnyExecutor::Kubernetes(_) => "job",
            executor::AnyExecutor::Katib(_) => "katib",
            executor::AnyExecutor::Ray(_) => "ray-job",
        });
        name.push('-');
    }

    name
}
