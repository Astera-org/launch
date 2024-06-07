use clap::{Args, Parser, Subcommand, ValueEnum};
use constcat::concat;
use home::home_dir;
use launch::{
    build,
    execution::{self, ExecutionArgs, ExecutionBackend},
    git, kubectl, tailscale,
};
use log::{debug, warn};

const DATABRICKSCFG_NAME: &str = "databrickscfg";
const DATABRICKSCFG_MOUNT: &str = "/root/.databrickscfg";

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Args)]
struct SubmitArgs {
    /// The minimum number of GPUs required to execute the work.
    #[arg(long = "gpus", default_value_t = 0)]
    gpus: u32,

    #[arg(long = "workers", default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    workers: u32,

    #[arg(long = "allow-dirty", default_value_t)]
    allow_dirty: bool,

    #[arg(long = "allow-unpushed", default_value_t)]
    allow_unpushed: bool,

    #[arg(long = "databrickscfg-mode", value_enum, default_value_t, help = concat!("Control whether a secret named \"", DATABRICKSCFG_NAME, "\" should be created from the submitting machine and mounted as a file at \"", DATABRICKSCFG_MOUNT, "\" through a volume in the container of the submitted job."))]
    databrickscfg_mode: DatabricksCfgMode,

    #[arg(required = true, last = true)]
    command: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
enum DatabricksCfgMode {
    /// The databrickscfg secret will be created and attached to the container if possible.
    #[default]
    Auto,
    /// The databrickscfg secret is required.
    Require,
    /// The databrickscfg secret should be omitted.
    Omit,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Submit work to the cluster
    #[command(arg_required_else_help = true)]
    Submit(SubmitArgs),

    /// List works submitted to the cluster
    List,
    /// Follow the logs
    #[command(arg_required_else_help = true)]
    Logs { pod_name: String },
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Submit(args) => {
            submit(args)?;
        }
        Commands::List => {
            todo!();
        }
        Commands::Logs { .. } => {
            todo!();
        }
    }

    Ok(())
}

fn get_user() -> Result<String, Box<dyn std::error::Error>> {
    let launched_by_user = tailscale::get_user()?;
    Ok(if launched_by_user.contains('@') {
        // The Tailscale login name refers to a person.
        launched_by_user
    } else {
        // The Tailscale login name refers to a machine, use the OS username instead.
        whoami::username()
    })
}

struct GitInfo {
    commit_hash: String,
    is_clean: bool,
    is_pushed: bool,
}

fn git_info() -> Result<GitInfo, Box<dyn std::error::Error>> {
    let commit_hash = git::commit_hash()?;
    debug!("git commit hash: {commit_hash}");

    let is_clean = git::is_clean()?;
    debug!("git is clean: {is_clean}");

    let is_pushed = {
        git::fetch()?;
        git::exists_on_any_remote(&commit_hash)?
    };
    debug!("git is pushed: {is_pushed}");

    Ok(GitInfo {
        commit_hash,
        is_clean,
        is_pushed,
    })
}

fn submit(args: SubmitArgs) -> Result<(), Box<dyn std::error::Error>> {
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
    let job_namespace = "launch";

    if command.is_empty() {
        return Err("Please provide the command to run".into());
    }

    let launched_by_user = get_user()?;
    debug!("launched_by_user: {launched_by_user:?}");

    let launched_by_hostname = whoami::fallible::hostname()?;
    debug!("launched_by_hostname: {launched_by_hostname:?}");

    let git_info = git_info()?;

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

    let kubectl =
        kubectl::Kubectl::new("https://berkeley-tailscale-operator.taila1eba.ts.net".to_string());

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
                job_namespace,
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
        launch::kubectl::to_rfc_1123_label_lossy(&launched_by_user).ok_or_else(|| format!(
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
        job_namespace,
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

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(error) = run() {
        const BOLD_RED: &str = "\x1b[1;31m";
        const BOLD: &str = "\x1b[1m";
        const RESET: &str = "\x1b[0m";
        eprintln!("{BOLD_RED}error{RESET}{BOLD}:{RESET} {error}");
        std::process::exit(1);
    }
}
