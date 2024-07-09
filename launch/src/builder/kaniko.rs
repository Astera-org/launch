use super::{BuildArgs, BuildOutput, Builder, Result};

pub struct KanikoBuilder;

impl Builder for KanikoBuilder {
    fn build(&self, _args: BuildArgs) -> Result<BuildOutput> {
        todo!("implement remote build backend")
    }
}
