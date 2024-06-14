use super::{BuildArgs, BuildBackend, BuildOutput, Result};

pub struct RemoteBuildBackend;

impl BuildBackend for RemoteBuildBackend {
    fn build(&self, _args: BuildArgs) -> Result<BuildOutput> {
        todo!("implement remote build backend")
    }
}
