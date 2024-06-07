use std::error::Error;

use crate::process;

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

/// Returns the commit hash.
pub fn commit_hash() -> Result<String> {
    let output = process::command!("git", "rev-parse", "HEAD").output()?;

    Ok(std::str::from_utf8(&output.stdout)?.trim().to_owned())
}

pub fn fetch() -> Result<()> {
    process::command!("git", "fetch").output()?;
    Ok(())
}

/// Checks if the commit is a part of any branch on any remote. Before calling this function, you
/// will probably want to call [`fetch`].
pub fn exists_on_any_remote(commit_hash: &str) -> Result<bool> {
    let output =
        process::command!("git", "branch", "--remote", "--contains", commit_hash).output()?;

    Ok(!std::str::from_utf8(&output.stdout)?.trim().is_empty())
}

/// Returns true if there are no uncommitted or untracked files, and false otherwise.
pub fn is_clean() -> Result<bool> {
    let output = process::command!("git", "status", "--porcelain").output()?;

    Ok(std::str::from_utf8(&output.stdout)?.trim().is_empty())
}
