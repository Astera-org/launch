mod docker;
mod kaniko;

pub use docker::*;
pub use kaniko::*;

use crate::Result;

pub struct BuildArgs<'a> {
    pub git_commit_hash: &'a str,
    pub image_name_with_tag: &'a str,
}

pub struct BuildOutput {
    pub image_digest: String,
    pub entrypoint: Vec<String>,
}

pub trait Builder {
    fn build(&self, args: BuildArgs) -> Result<BuildOutput>;
}
