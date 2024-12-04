use core::fmt;

use crate::{process, Result};

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
    pub image_tag: &'a str,
    pub platform: Platform,
}

pub struct BuildOutput {
    pub image_digest: String,
}

pub fn build_and_push(args: BuildArgs) -> Result<BuildOutput> {
    let metadata_filepath = crate::temp_path::tmp_json_path();

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
        "--platform",
        args.platform.as_str(),
        // https://github.com/opencontainers/image-spec/blob/main/annotations.md
        "--annotation",
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

pub fn entrypoint(image_ref: &str) -> Result<Option<Vec<String>>> {
    let output = process::command!(
        "docker",
        "inspect",
        "--format",
        "{{json .Config.Entrypoint}}",
        image_ref,
    )
    .output()?;

    Ok(Some(serde_json::from_slice(&output.stdout)?))
}
