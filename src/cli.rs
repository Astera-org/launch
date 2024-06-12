mod common;
mod list;
mod submit;

use crate::Result;
use clap::{Parser, Subcommand};
use constcat::concat;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
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
                submit::submit(args)?;
            }
            Commands::List => {
                list::list()?;
            }
            Commands::Logs { .. } => {
                todo!();
            }
        }

        Ok(())
    }
}
