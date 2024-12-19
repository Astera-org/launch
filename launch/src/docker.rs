use core::fmt;

use log::debug;

use crate::{container_image::ContainerImage, process, Result};

/// Partial implementation of the JSON emitted by the `--metadata-file` option of `docker build`.
/// See https://docs.docker.com/reference/cli/docker/buildx/build/#metadata-file.
#[derive(serde::Deserialize)]
struct MetadataFile {
    #[serde(rename = "containerimage.digest")]
    containerimage_digest: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Platform {
    LinuxAmd64,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::LinuxAmd64 => "linux/amd64",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub struct BuildArgs<'a> {
    pub git_commit_hash: &'a str,
    pub image: ContainerImage<'a>,
    pub platform: Platform,
}

pub struct BuildOutput<'a> {
    pub image: ContainerImage<'a>,
}

pub fn build_and_push(args: BuildArgs) -> Result<BuildOutput> {
    let BuildArgs {
        image,
        git_commit_hash,
        platform,
    } = args;
    debug!("Building image: {:?}", image);

    let metadata_filepath = crate::temp_path::tmp_json_path();
    process::command!(
        "docker",
        "buildx",
        "build",
        ".",
        format!("--metadata-file={}", metadata_filepath.display()),
        format!("--tag={}", image.image_url()),
        format!("--build-arg=COMMIT_HASH={git_commit_hash}"),
        format!("--platform={platform}"),
        // https://github.com/opencontainers/image-spec/blob/main/annotations.md
        format!("--annotation=org.opencontainers.image.revision={git_commit_hash}"),
        "--push",
    )
    .status()?;

    let metadata_string = std::fs::read_to_string(&metadata_filepath)?;
    let metadata: MetadataFile = serde_json::from_str(&metadata_string)?;
    let mut image = image.clone();
    image.digest = Some(metadata.containerimage_digest);
    Ok(BuildOutput { image })
}
