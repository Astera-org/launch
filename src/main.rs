use std::collections::HashMap;

use clap::{Args, Parser, Subcommand, ValueEnum};
use constcat::concat;
use home::home_dir;
use itertools::Itertools;
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

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

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

fn run() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Submit(args) => {
            submit(args)?;
        }
        Commands::List => {
            list()?;
        }
        Commands::Logs { .. } => {
            todo!();
        }
    }

    Ok(())
}

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

struct GitInfo {
    commit_hash: String,
    is_clean: bool,
    is_pushed: bool,
}

fn git_info() -> Result<GitInfo> {
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

const LAUNCH_NAMESPACE: &str = "launch";

fn submit(args: SubmitArgs) -> Result<()> {
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

    let kubectl = kubectl();

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
                LAUNCH_NAMESPACE,
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
        job_namespace: LAUNCH_NAMESPACE,
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

fn list() -> Result<()> {
    use comfy_table::{Attribute, Cell, ContentArrangement, Table};
    use launch::time_ext::OffsetDateTimeExt;

    let kubectl = kubectl();

    let jobs = kubectl.jobs(LAUNCH_NAMESPACE)?;
    let rayjobs = kubectl.rayjobs(LAUNCH_NAMESPACE)?;

    let mut map: HashMap<String, (Option<kubectl::Job>, Option<kubectl::RayJob>)> =
        HashMap::with_capacity(jobs.len() + rayjobs.len());

    for job in jobs {
        assert!(map
            .entry(job.metadata.name.clone())
            .or_default()
            .0
            .replace(job)
            .is_none());
    }

    for rayjob in rayjobs {
        assert!(map
            .entry(rayjob.metadata.name.clone())
            .or_default()
            .1
            .replace(rayjob)
            .is_none());
    }

    struct Row {
        name: String,
        creation_timestamp: time::OffsetDateTime,
        launched_by_user: Option<String>,
        job_status: Option<String>,
        rayjob_status: Option<String>,
    }

    let rows = {
        let mut rows: Vec<Row> = map
            .into_iter()
            .map(|(name, (job, rayjob))| Row {
                name,
                creation_timestamp: match (&job, &rayjob) {
                    (Some(job), Some(rayjob)) => job
                        .metadata
                        .creation_timestamp
                        .min(rayjob.metadata.creation_timestamp),
                    (Some(job), None) => job.metadata.creation_timestamp,
                    (None, Some(rayjob)) => rayjob.metadata.creation_timestamp,
                    (None, None) => unreachable!(
                        "each entry in the hashmap should have at least a job or a rayjob"
                    ),
                },
                launched_by_user: job
                    .as_ref()
                    .and_then(|job| job.metadata.annotations.get("launched_by_user"))
                    .or(rayjob
                        .as_ref()
                        .and_then(|rayjob| rayjob.metadata.annotations.get("launched_by_user")))
                    .cloned(),
                job_status: job.map(|job| {
                    job.status
                        .conditions
                        .iter()
                        .map(|condition| match &condition.reason {
                            Some(reason) => format!("{}: {reason}", &condition.r#type),
                            None => condition.r#type.to_string(),
                        })
                        .join("\n")
                }),
                rayjob_status: rayjob.map(|rayjob| rayjob.status.job_deployment_status),
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.creation_timestamp.cmp(&b.creation_timestamp).reverse());
        rows
    };

    // The `Accessor` type and `accessor` function aid type inference. The type of an array is inferred from the first
    // element. Without the type annotation, the compiler treats the first element's accessor as a closure and not a
    // function pointer. Every closure compiles down to it's own unique type. The elements of an array must all be of
    // the same type. With more than 1 element, compilation fails.  We could also do it by specifying the type of
    // `columns`, but we can not infer the number of items in the array. See
    // https://github.com/rust-lang/rust/issues/85077.
    type Accessor = fn(&Row) -> Result<Option<String>>;

    fn accessor(f: Accessor) -> Accessor {
        f
    }

    fn format_date(value: time::OffsetDateTime) -> Result<String> {
        let fd = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

        Ok(value.to_local()?.format(fd)?)
    }

    fn format_offset(value: time::UtcOffset) -> Result<String> {
        let fd = time::macros::format_description!("[offset_hour sign:mandatory]:[offset_minute]");
        Ok(value.format(fd)?)
    }

    // The code below keeps column names together with a function that produces the value from the row data for that
    // column. Unfortunately, it does cause additional work. Perhaps some procedural macro machinery for defining table
    // row types with field annotations for headers and formatting implementations would be better.
    let columns = [
        (
            "name".to_string(),
            accessor(|row| Ok(Some(row.name.clone()))),
        ),
        (
            format!(
                "created ({})",
                format_offset(launch::time_ext::local_offset()?)?
            ),
            accessor(|row| Ok(Some(format_date(row.creation_timestamp)?))),
        ),
        (
            "Job status".to_string(),
            accessor(|row| Ok(row.job_status.clone())),
        ),
        (
            "RayJob status".to_string(),
            accessor(|row| Ok(row.rayjob_status.clone())),
        ),
        (
            "launched by".to_string(),
            accessor(|row| {
                Ok(row
                    .launched_by_user
                    .as_deref()
                    .and_then(|user| user.split('@').next().map(str::to_string)))
            }),
        ),
    ];

    let (column_names, accessors): (Vec<_>, Vec<_>) = columns.into_iter().unzip();

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            column_names
                .into_iter()
                .map(|name| Cell::new(name).add_attribute(Attribute::Bold)),
        );

    for row in rows {
        // We need to collect here because we need to consume the iterator to filter out errors before we can pass it to
        // `Table::add_row` since it does not accept a Result.
        table.add_row({
            accessors
                .iter()
                .map(|f| f(&row))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|value| value.unwrap_or_default())
        });
    }

    println!("{table}");

    Ok(())
}

fn kubectl() -> kubectl::Kubectl {
    kubectl::Kubectl::new("https://berkeley-tailscale-operator.taila1eba.ts.net".to_string())
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
