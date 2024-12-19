mod docker;
mod kaniko;

pub use docker::*;
pub use kaniko::*;

use crate::{
    container_image::ContainerImage,
    git::{self},
    Result,
};

pub struct BuildArgs<'a> {
    pub git_info: &'a git::GitInfo,
    pub image: &'a ContainerImage<'a>,
}

pub struct BuildOutput<'a> {
    pub image: ContainerImage<'a>,
}

pub trait Builder<'a> {
    fn build(&self, args: BuildArgs<'a>) -> Result<BuildOutput<'a>>;
}
