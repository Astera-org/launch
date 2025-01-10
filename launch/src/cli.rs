mod common;
mod list;
mod submit;

use clap::{Parser, Subcommand, ValueEnum};
use constcat::concat;
use log::{error, warn};

use crate::{kubectl::Kubectl, Result};

#[derive(Debug, Default, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ClusterContext {
    /// Refers to https://berkeley-tailscale-operator.taila1eba.ts.net
    #[default]
    Berkeley,

    /// Refers to https://staging-tailscale-operator.taila1eba.ts.net
    Staging,

    /// Refers to https://voltage-park-tailscale-operator.taila1eba.ts.net
    VoltagePark,
}

impl ClusterContext {
    pub const fn cluster_url(&self) -> &'static str {
        match self {
            ClusterContext::Berkeley => "https://berkeley-tailscale-operator.taila1eba.ts.net",
            ClusterContext::Staging => "https://staging-tailscale-operator.taila1eba.ts.net",
            ClusterContext::VoltagePark => {
                "https://voltage-park-tailscale-operator.taila1eba.ts.net"
            }
        }
    }

    pub const fn headlamp_url(&self) -> &'static str {
        match self {
            ClusterContext::Berkeley => "https://berkeley-headlamp.taila1eba.ts.net",
            ClusterContext::Staging => "https://staging-headlamp.taila1eba.ts.net",
            ClusterContext::VoltagePark => "https://voltage-park-headlamp.taila1eba.ts.net",
        }
    }

    pub const fn katib_url(&self) -> &'static str {
        match self {
            ClusterContext::Berkeley => "http://berkeley-katib.taila1eba.ts.net",
            ClusterContext::Staging => "http://staging-katib.taila1eba.ts.net",
            ClusterContext::VoltagePark => "http://voltage-park-katib.taila1eba.ts.net",
        }
    }

    pub const fn container_registry_host(&self) -> &'static str {
        match self {
            ClusterContext::Berkeley => "berkeley-docker.taila1eba.ts.net",
            ClusterContext::Staging => "staging-docker.taila1eba.ts.net",
            ClusterContext::VoltagePark => "voltage-park-docker.taila1eba.ts.net",
        }
    }

    pub fn kubectl(&self) -> Kubectl {
        Kubectl::new(self.cluster_url())
    }
}

#[derive(Debug, Parser)]
#[command(version = crate::version::VERSION, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long = "context", global = true, value_enum, default_value_t)]
    context: ClusterContext,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Submit work to the cluster
    #[command(arg_required_else_help = true)]
    Submit(submit::SubmitArgs),

    /// List works submitted to the cluster
    List(list::ListArgs),
    /// Follow the logs
    #[command(arg_required_else_help = true)]
    Logs { pod_name: String },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let latest_version_lock = std::sync::Arc::new(std::sync::Mutex::new(None));

        // Perform the latest version check on SIGINT for commands that don't end quickly, such as
        // those tailing logs.
        ctrlc::set_handler({
            let latest_version_lock = std::sync::Arc::clone(&latest_version_lock);
            move || latest_version_check(&latest_version_lock)
        })
        .expect("Failed to set Ctrl-C handler");

        // Query the latest version on a separate thread so that it does not block execution of the
        // user's command. This avoids a long wait when the network is not available or slow.
        std::thread::Builder::new()
            .name("version_check".to_string())
            .spawn({
                let latest_version_lock = std::sync::Arc::clone(&latest_version_lock);
                move || {
                    if let Some(latest_version) = query_latest_version() {
                        latest_version_lock.lock().unwrap().replace(latest_version);
                    }
                }
            })
            .unwrap();

        match self.command {
            Commands::Submit(args) => {
                submit::submit(&self.context, args)?;
            }
            Commands::List(args) => {
                list::list(&self.context, args)?;
            }
            Commands::Logs { .. } => {
                todo!();
            }
        }

        latest_version_check(&latest_version_lock);

        Ok(())
    }
}

fn query_latest_version() -> Option<semver::Version> {
    let output = std::process::Command::new("pixi")
        .args([
            "search",
            "--channel=https://repo.prefix.dev/obelisk",
            "--limit=1",
            "launch",
        ])
        .output()
        .inspect_err(|err| error!("Failed to invoke pixi search for launch version check: {err}"))
        .ok()?;

    let stdout = std::str::from_utf8(&output.stdout)
        .inspect_err(|err| {
            error!("Failed to parse pixi search output as UTF-8 for launch version check: {err}")
        })
        .ok()?;

    // This implementation allows for the rows in the table output by pixi search to be reordered.
    let mut name_matches = false;
    let mut version = None;
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next();
        match key {
            Some("Name") => {
                let Some("launch") = parts.next() else {
                    error!("Failed to parse pixi search output for launch version check: expected `Name launch` but got: {line}");
                    return None;
                };
                name_matches = true;
            }
            Some("Version") => {
                let Some(value) = parts
                    .next()
                    .and_then(|value| semver::Version::parse(value).ok())
                else {
                    error!("Failed to parse pixi search output for launch version check: expected `Version <version>` but got: {line}");
                    return None;
                };
                version = Some(value);
            }
            _ => {
                // Unrecognized line.
            }
        }

        if name_matches && version.is_some() {
            break;
        }
    }

    if !name_matches {
        error!("Failed to parse pixi search output for launch version check: expected `Name launch` but found nothing:\n{stdout}");
        return None;
    }

    let Some(version) = version else {
        error!("Failed to parse pixi search output for launch version check: expected `Version <version>` but found nothing:\n{stdout}");
        return None;
    };

    Some(version)
}

/// Prints a warning if the latest_version has been set before this method is called, and the
/// latest_version is newer than the current version.
fn latest_version_check(
    latest_version_lock: &std::sync::Arc<std::sync::Mutex<Option<semver::Version>>>,
) {
    if let Some(latest_version) = latest_version_lock.lock().unwrap().take() {
        let current_version = semver::Version::parse(crate::version::VERSION).unwrap();
        if latest_version > current_version {
            warn!("A newer version of launch is available, install it with `pixi global install --channel https://repo.prefix.dev/obelisk launch=={latest_version}`");
        }
    }
}
