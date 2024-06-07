mod local;
mod remote;

pub use local::*;
pub use remote::*;

pub struct BuildArgs<'a> {
    pub git_commit_hash: &'a str,
    pub image_tag: &'a str,
}

pub struct BuildOutput {
    pub image_digest: String,
}

pub trait BuildBackend {
    fn build(&self, args: BuildArgs) -> Result<BuildOutput>;
}

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
