use log::debug;

use crate::{process, Result};

/// Returns the commit hash.
fn commit_hash() -> Result<String> {
    let output = process::command!("git", "rev-parse", "HEAD").output()?;

    Ok(std::str::from_utf8(&output.stdout)?.trim().to_owned())
}

fn fetch() -> Result<()> {
    process::command!("git", "fetch").output()?;
    Ok(())
}

/// Checks if the commit is a part of any branch on any remote. Before calling this function, you
/// will probably want to call [`fetch`].
fn exists_on_any_remote(commit_hash: &str) -> Result<bool> {
    let output =
        process::command!("git", "branch", "--remote", "--contains", commit_hash).output()?;

    Ok(!std::str::from_utf8(&output.stdout)?.trim().is_empty())
}

/// Returns true if there are no uncommitted or untracked files, and false otherwise.
fn is_clean() -> Result<bool> {
    let output = process::command!("git", "status", "--porcelain").output()?;

    Ok(std::str::from_utf8(&output.stdout)?.trim().is_empty())
}

pub struct GitInfo {
    pub commit_hash: String,
    pub is_clean: bool,
    pub is_pushed: bool,
}

pub fn info() -> Result<GitInfo> {
    let commit_hash = commit_hash()?;
    debug!("git commit hash: {commit_hash}");

    let is_clean = is_clean()?;
    debug!("git is clean: {is_clean}");

    let is_pushed = {
        fetch()?;
        exists_on_any_remote(&commit_hash)?
    };
    debug!("git is pushed: {is_pushed}");

    Ok(GitInfo {
        commit_hash,
        is_clean,
        is_pushed,
    })
}
