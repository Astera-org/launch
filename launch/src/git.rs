use core::fmt;
use std::{
    ops::{self},
    path::{Path, PathBuf},
};

use crate::{process, Result};

fn git_dir() -> Result<PathBuf> {
    let output = process::command!("git", "rev-parse", "--show-toplevel").output()?;
    Ok(Path::new(std::str::from_utf8(&output.stdout)?.trim()).to_owned())
}

/// Returns the commit hash.
fn commit_hash() -> Result<String> {
    let output = process::command!("git", "rev-parse", "HEAD").output()?;
    Ok(std::str::from_utf8(&output.stdout)?.trim().to_owned())
}

/// Returns the push location of the current branch if configured.
fn push_branch() -> Result<Option<RemoteBranch>> {
    let output = process::command!(
        "git",
        "rev-parse",
        "--abbrev-ref",
        "--symbolic-full-name",
        "@{push}"
    )
    .try_output()?;
    Ok(if output.status.success() {
        Some(RemoteBranch::new(
            std::str::from_utf8(&output.stdout)?.trim().to_owned(),
        )?)
    } else {
        None
    })
}

/// Returns the url for a remote.
fn remote_url(remote: &str) -> Result<String> {
    let output = process::command!("git", "remote", "get-url", remote).output()?;
    Ok(std::str::from_utf8(&output.stdout)?.trim().to_owned())
}

fn fetch() -> Result<()> {
    process::command!("git", "fetch").output()?;
    Ok(())
}

/// Checks if the specified branch contains the specified commit. Before calling this function, you will probably want
/// to call [`fetch`].
fn is_pushed(remote_branch: &RemoteBranch, commit_hash: &str) -> Result<bool> {
    let output = process::command!(
        "git",
        "branch",
        "--remote",
        "--list",
        remote_branch.as_str(),
        "--contains",
        commit_hash
    )
    .output()?;

    Ok(!std::str::from_utf8(&output.stdout)?.trim().is_empty())
}

/// Returns true if there are no uncommitted or untracked files, and false otherwise.
fn is_clean() -> Result<bool> {
    let output = process::command!("git", "status", "--porcelain").output()?;

    Ok(std::str::from_utf8(&output.stdout)?.trim().is_empty())
}

pub struct GitInfo {
    pub dir: PathBuf,
    pub commit_hash: String,
    #[allow(dead_code)]
    // The url of the push remote.
    pub push_remote_url: Option<String>,
    pub is_clean: bool,
    pub is_pushed: bool,
}

pub fn info() -> Result<GitInfo> {
    let dir = git_dir()?;
    let commit_hash = commit_hash()?;
    let is_clean = is_clean()?;
    let push_branch = push_branch()?;
    let is_pushed = match push_branch.as_ref() {
        Some(branch) => {
            fetch()?;
            is_pushed(branch, &commit_hash)?
        }
        None => false,
    };
    let push_remote_url = push_branch
        .as_ref()
        .map(RemoteBranch::remote)
        .map(remote_url)
        .transpose()?;
    Ok(GitInfo {
        dir,
        commit_hash,
        push_remote_url,
        is_clean,
        is_pushed,
    })
}

pub fn is_full_git_commit_hash(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 40 && bytes.iter().all(u8::is_ascii_hexdigit)
}

pub struct RemoteBranch {
    value: String,
    split_at: usize,
}

impl RemoteBranch {
    fn new(value: String) -> Result<Self> {
        let split_at = value.find('/').ok_or("expected a slash")?;
        Ok(Self { value, split_at })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    fn split(&self) -> (&str, &str) {
        let (remote, slash_branch) = self.value.split_at(self.split_at);
        (remote, slash_branch.split_at(1).1)
    }

    pub fn remote(&self) -> &str {
        self.split().0
    }

    // Part of the API but currently unused.
    #[allow(unused)]
    pub fn branch(&self) -> &str {
        self.split().1
    }
}

impl AsRef<str> for RemoteBranch {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RemoteBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl ops::Deref for RemoteBranch {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_branch_new_valid() {
        let branch = RemoteBranch::new("origin/main".to_string()).unwrap();
        assert_eq!(branch.value, "origin/main");
        assert_eq!(branch.split_at, 6);
    }

    #[test]
    fn test_remote_branch_new_invalid() {
        let result = RemoteBranch::new("invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_remote_branch_split() {
        let branch = RemoteBranch::new("origin/feature-branch".to_string()).unwrap();
        assert_eq!(branch.remote(), "origin");
        assert_eq!(branch.branch(), "feature-branch");
    }
}
