use std::error::Error;

use crate::process;

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

pub struct BuildArgs<'a> {
    pub git_commit_hash: &'a str,
    pub image_tag: &'a str,
}

pub struct BuildOutput {
    pub image_digest: String,
}

pub fn build_and_push(args: BuildArgs) -> Result<BuildOutput, Box<dyn Error>> {
    let metadata_filepath = tmp_json_path();

    process::command!(
        "docker",
        "buildx",
        "build",
        ".",
        "--metadata-file",
        metadata_filepath,
        "--tag",
        args.image_tag,
        "--build-arg",
        format!("COMMIT_HASH={}", args.git_commit_hash),
        "--annotation",
        // https://github.com/opencontainers/image-spec/blob/main/annotations.md
        format!(
            "org.opencontainers.image.revision={revision}",
            revision = args.git_commit_hash
        ),
        "--push",
    )
    .status()?;

    let metadata_string = std::fs::read_to_string(&metadata_filepath)?;

    let metadata: MetadataFile = serde_json::from_str(&metadata_string)?;

    Ok(BuildOutput {
        image_digest: metadata.containerimage_digest,
    })
}
