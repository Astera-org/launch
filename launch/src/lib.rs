pub(crate) mod ansi;
pub(crate) mod bash_escape;
pub(crate) mod builder;
pub(crate) mod docker;
pub(crate) mod executor;
pub(crate) mod git;
pub(crate) mod kubectl;
pub(crate) mod process;
pub(crate) mod tailscale;
pub(crate) mod temp_path;
pub(crate) mod unit;
pub(crate) mod user_host;
pub(crate) mod version;

pub mod cli;

pub(crate) type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
    std::result::Result<T, E>;
