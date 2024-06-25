mod common;
mod list;
mod submit;

use clap::{Parser, Subcommand, ValueEnum};
use constcat::concat;

use crate::{kubectl::Kubectl, Result};

#[derive(Debug, Default, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ClusterContext {
    /// Refers to https://staging-tailscale-operator.taila1eba.ts.net
    Staging,

    /// Refers to https://berkeley-tailscale-operator.taila1eba.ts.net
    #[default]
    Berkeley,
}

impl ClusterContext {
    pub const fn cluster_url(&self) -> &'static str {
        match self {
            ClusterContext::Staging => "https://staging-tailscale-operator.taila1eba.ts.net",
            ClusterContext::Berkeley => "https://berkeley-tailscale-operator.taila1eba.ts.net",
        }
    }

    pub const fn headlamp_url(&self) -> &'static str {
        match self {
            ClusterContext::Staging => "https://staging-headlamp.taila1eba.ts.net",
            ClusterContext::Berkeley => "https://berkeley-headlamp.taila1eba.ts.net",
        }
    }

    pub const fn docker_host(&self) -> &'static str {
        match self {
            ClusterContext::Staging => "staging-docker.taila1eba.ts.net",
            ClusterContext::Berkeley => "berkeley-docker.taila1eba.ts.net",
        }
    }

    pub const fn docker_host_inside_cluster(&self) -> &'static str {
        "astera-infra.com"
    }

    pub fn kubectl(&self) -> Kubectl {
        Kubectl::new(self.cluster_url())
    }
}

#[derive(Debug, Parser)]
#[command(version, about)]
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
    List,
    /// Follow the logs
    #[command(arg_required_else_help = true)]
    Logs { pod_name: String },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Commands::Submit(args) => {
                submit::submit(&self.context, args)?;
            }
            Commands::List => {
                list::list(&self.context)?;
            }
            Commands::Logs { .. } => {
                todo!();
            }
        }

        Ok(())
    }
}
