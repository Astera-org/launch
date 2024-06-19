pub(crate) mod build;
pub(crate) mod docker;
pub(crate) mod execution;
pub(crate) mod git;
pub(crate) mod kubectl;
pub(crate) mod process;
pub(crate) mod tailscale;
pub(crate) mod temp_path;
pub(crate) mod time_ext;
pub(crate) mod unit;
pub(crate) mod user_host;

pub mod cli;

pub(crate) type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
    std::result::Result<T, E>;
