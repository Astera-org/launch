use crate::{as_ref, process};
use std::error::Error;

fn docker() -> process::Command {
    process::Command::new("docker")
}

fn tmp_json_path() -> std::path::PathBuf {
    use rand::distributions::{Alphanumeric, DistString};

    const DIR: &str = "/tmp/";
    const EXT: &str = ".json";
    const LEN: usize = 16;

    let mut path = String::with_capacity(DIR.len() + LEN + EXT.len());
    path.push_str(DIR);
    Alphanumeric.append_string(&mut rand::thread_rng(), &mut path, LEN);
    path.push_str(EXT);
    path.into()
}

/// Partial implementation of the JSON emitted by the `--metadata-file` option of `docker build`.
/// See https://docs.docker.com/reference/cli/docker/buildx/build/#metadata-file.
#[derive(serde::Deserialize)]
struct MetadataFile {
    #[serde(rename = "containerimage.digest")]
    containerimage_digest: String,
}

pub struct BuildOutput {
    pub digest: String,
}

pub fn docker_build_and_push(tag: &str) -> Result<BuildOutput, Box<dyn Error>> {
    let metadata_filepath = tmp_json_path();

    docker()
        .args(as_ref![
            "buildx",
            "build",
            ".",
            "--metadata-file",
            metadata_filepath,
            "--tag",
            tag,
            "--push",
        ])
        .status()?;

    let metadata_string = std::fs::read_to_string(&metadata_filepath)?;

    let metadata: MetadataFile = serde_json::from_str(&metadata_string)?;

    Ok(BuildOutput {
        digest: metadata.containerimage_digest,
    })
}
