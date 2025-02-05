use super::{BuildArgs, BuildOutput, Builder};
use crate::{docker, Result};

pub struct DockerBuilder;

impl Builder for DockerBuilder {
    fn build<'a>(&'a self, args: BuildArgs<'a>) -> Result<BuildOutput> {
        // This conversion is necessary because the build arguments for the backend may differ from the
        // build arguments accepted by the docker command line abstraction.
        let docker_build_output = docker::build_and_push(docker::BuildArgs {
            git_commit_hash: &args.git_info.commit_hash,
            image: args.image,
            platform: docker::Platform::LinuxAmd64,
        })?;
        Ok(BuildOutput {
            digest: docker_build_output.digest,
        })
    }
}
