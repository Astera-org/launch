use log::debug;

use super::{BuildArgs, BuildOutput, Builder};
use crate::{docker, Result};

pub struct DockerBuilder;

// This conversion is necessary because the build arguments for the backend may differ from the
// build arguments accepted by the docker command line abstraction.
fn into_docker_args(args: BuildArgs) -> docker::BuildArgs {
    let BuildArgs {
        git_commit_hash,
        image_name_with_tag: image_tag,
    } = args;
    docker::BuildArgs {
        git_commit_hash,
        image_tag,
        platform: docker::Platform::LinuxAmd64,
    }
}

// This conversion is necessary because the build output for the docker command line abstraction may
// differ from the build output returned by the build backend.
fn from_docker_output(output: docker::BuildOutput, entrypoint: Vec<String>) -> BuildOutput {
    BuildOutput {
        image_digest: output.image_digest,
        entrypoint,
    }
}

impl Builder for DockerBuilder {
    fn build(&self, args: BuildArgs) -> Result<BuildOutput> {
        let image_tag = args.image_name_with_tag;
        let docker_build_output = docker::build_and_push(into_docker_args(args))?;
        let entrypoint = docker::entrypoint(image_tag)?.unwrap_or_default();
        let output = from_docker_output(docker_build_output, entrypoint);
        debug!("image_digest: {:?}", output.image_digest);
        Ok(output)
    }
}
