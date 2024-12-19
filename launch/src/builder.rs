mod docker;
mod kaniko;

use container_image_name::ImageNameRef;
pub use docker::*;
pub use kaniko::*;

use crate::{
    git::{self},
    Result,
};

pub struct BuildArgs<'a> {
    pub git_info: &'a git::GitInfo,
    pub image: ImageNameRef<'a>,
}

pub struct BuildOutput {
    pub digest: String,
}

pub trait Builder {
    fn build<'a>(&'a self, args: BuildArgs<'a>) -> Result<BuildOutput>;
}
