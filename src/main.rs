use constcat::concat;
use home::home_dir;
use launch::{ContainerStatus, PodPhase, PodPhasePendingReason, PodPhaseRunningReason, PodStatus};
use log::{debug, info, warn};

use clap::{Parser, Subcommand, ValueEnum};

const DATABRICKSCFG_NAME: &str = "databrickscfg";
const DATABRICKSCFG_MOUNT: &str = "/root/.databrickscfg";

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    Submit {
        /// The minimum number of GPUs required to execute the work.
        #[arg(long = "gpus", default_value = "0")]
        gpus: u32,

        #[arg(long = "databrickscfg-mode", value_enum, default_value_t, help = concat!("Control whether a secret named \"", DATABRICKSCFG_NAME, "\" should be created from the submitting machine and mounted as a file at \"", DATABRICKSCFG_MOUNT, "\" through a volume in the container of the submitted job."))]
        databrickscfg_mode: DatabricksCfgMode,

        #[arg(required = true, last = true)]
        command: Vec<String>,
    },
    /// List works submitted to the cluster
    List,
    /// Follow the logs
    #[command(arg_required_else_help = true)]
    Logs { pod_name: String },
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Submit {
            gpus,
            databrickscfg_mode: databrickscfg,
            command,
        } => {
            submit(gpus, databrickscfg, command)?;
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
    let launched_by_user = launch::tailscale_get_user()?;
    Ok(if launched_by_user.contains('@') {
        // The Tailscale login name refers to a person.
        launched_by_user
    } else {
        // The Tailscale login name refers to a machine, use the OS username instead.
        whoami::username()
    })
}

fn submit(
    gpus: u32,
    databrickscfg_mode: DatabricksCfgMode,
    command: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let tag = format!("{image_registry_outside_cluster}/{image_name}:latest");
    let image_digest = launch::docker_build_and_push(&tag)?.digest;
    debug!("image_digest: {image_digest:?}");

    let home_dir = home_dir().ok_or("failed to determine home directory")?;

    let kubectl =
        launch::Kubectl::new("https://berkeley-tailscale-operator.taila1eba.ts.net".to_string());

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

    let generate_name = format!(
        "launch-{}-",
        launch::to_rfc_1123_label_lossy(&launched_by_user).ok_or_else(|| format!(
            "Failed to generated job name from tailscale username {launched_by_user:?}"
        ))?
    );

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

    let job_spec = serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "namespace": job_namespace,
            "generateName": generate_name,
            "annotations": {
                "launched_by_user": launched_by_user,
                "launched_by_hostname": launched_by_hostname
            }
        },
        "spec": {
            "template": {
                "metadata": {
                    "annotations": {
                        "launched_by_user": launched_by_user,
                        "launched_by_hostname": launched_by_hostname,
                    }
                },
                "spec": {
                    "containers": [
                        {
                            "name": "fluid",
                            "image": &format!("{image_registry_inside_cluster}/{image_name}@{image_digest}"),
                            "command": &command,
                            "env": [
                                {
                                    // Suppress warnings from GitPython (used by mlflow)
                                    // about the git executable not being available.
                                    "name": "GIT_PYTHON_REFRESH",
                                    "value": "quiet"
                                }
                            ],
                            "volumeMounts": volume_mounts,
                            "resources": {
                                "limits": {
                                    "nvidia.com/gpu": gpus,
                                }
                            }
                        }
                    ],
                    "volumes": volumes,
                    // Defines whether a container should be restarted until it 1) runs forever, 2)
                    // runs succesfully, or 3) has run once. We just want our command to run once
                    // and so we never restart.
                    "restartPolicy": "Never"
                }
            },
            // How many times to retry running the pod and all its containers, should any of them
            // fail.
            "backoffLimit": 0,
            "ttlSecondsAfterFinished": 86400
        }
    }).to_string();

    let job_name = {
        let job = kubectl.create_job(&job_spec)?;
        assert_eq!(job_namespace, job.namespace);
        job.job_name
    };

    debug!("job_namespace: {:?}", job_namespace);
    debug!("job_id: {:?}", job_name);
    info!(
        "Created job {:?}",
        format!("{headlamp_base_url}/c/main/jobs/{job_namespace}/{job_name}")
    );

    let pod_name = {
        let mut pods = kubectl.get_pods_for_job(job_namespace, &job_name)?;
        assert_eq!(pods.len(), 1);
        pods.pop().unwrap()
    };
    debug!("pod_namespace: {:?}", job_namespace);
    debug!("pod_id: {:?}", pod_name);
    info!(
        "Created pod {:?}",
        format!("{headlamp_base_url}/c/main/pods/{job_namespace}/{pod_name}")
    );

    info!("Waiting for pod logs to become available...");

    let mut status = kubectl.pod_status(job_namespace, &pod_name)?;
    debug!("Pod status: {status}");

    fn are_logs_available(status: &PodStatus) -> Option<bool> {
        match &status.phase {
            PodPhase::Pending(reason) => match reason.as_ref() {
                Some(PodPhasePendingReason::ContainerCreating) => None,
                Some(PodPhasePendingReason::PodScheduled) => None,
                Some(PodPhasePendingReason::Unschedulable) => Some(false),
                None => {
                    if status
                        .container_statuses
                        .iter()
                        .any(ContainerStatus::cannot_pull_image)
                    {
                        Some(false)
                    } else {
                        None
                    }
                }
            },
            PodPhase::Running(reason) => match reason.as_ref() {
                Some(PodPhaseRunningReason::Started) => Some(true),
                Some(PodPhaseRunningReason::ContainerCreating) => None,
                Some(PodPhaseRunningReason::PodInitializing) => None,
                None => Some(true),
            },
            PodPhase::Succeeded(_) => Some(true),
            PodPhase::Failed(_) => Some(false),
            PodPhase::Unknown(_) => Some(false),
        }
    }

    let logs_available = loop {
        if let Some(logs_available) = are_logs_available(&status) {
            break logs_available;
        }

        std::thread::sleep(std::time::Duration::from_secs(2));

        status = {
            let new_status = kubectl.pod_status(job_namespace, &pod_name)?;
            if new_status != status {
                debug!("Pod status: {new_status}");
            }
            new_status
        };
    };

    if !logs_available {
        return Err(format!(
            "Pod logs will not become available because it reached status {status}"
        )
        .into());
    }

    kubectl.follow_pod_logs(job_namespace, &pod_name)?;

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
