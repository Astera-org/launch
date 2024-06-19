use log::debug;

use super::{BuildArgs, BuildBackend, BuildOutput};
use crate::{docker, Result};

pub struct LocalBuildBackend;

// This conversion is necessary because the build arguments for the backend may differ from the
// build arguments accepted by the docker command line abstraction.
fn into_docker_args(args: BuildArgs) -> docker::BuildArgs {
    let BuildArgs {
        git_commit_hash,
        image_tag,
    } = args;
    docker::BuildArgs {
        git_commit_hash,
        image_tag,
    }
}

// This conversion is necessary because the build output for the docker command line abstraction may
// differ from the build output returned by the build backend.
fn from_docker_output(output: docker::BuildOutput) -> BuildOutput {
    let docker::BuildOutput { image_digest } = output;
    BuildOutput { image_digest }
}

impl BuildBackend for LocalBuildBackend {
    fn build(&self, args: BuildArgs) -> Result<BuildOutput> {
        let output = from_docker_output(docker::build_and_push(into_docker_args(args))?);
        debug!("image_digest: {:?}", output.image_digest);
        Ok(output)
    }
}
