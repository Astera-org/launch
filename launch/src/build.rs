mod local;
mod remote;

pub use local::*;

use crate::Result;

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
