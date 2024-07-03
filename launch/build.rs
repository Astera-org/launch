use std::{env, fs, io::BufRead, path::PathBuf};

fn git<'a, I: IntoIterator<Item = &'a str>>(args: I) -> std::process::Output {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .expect("unable to invoke git");
    assert!(output.status.success(), "git invocation failed");
    output
}

fn git_commit_hash() -> String {
    let output = git(["rev-parse", "--short", "HEAD"]);
    let mut lines = output.stdout.lines();
    let commit_hash = lines.next().unwrap().unwrap();
    assert!(lines.next().is_none());
    commit_hash
}

fn git_is_clean() -> bool {
    git(["status", "--porcelain"]).stdout.is_empty()
}

fn main() {
    let doing_release = option_env!("LAUNCH_RELEASE")
        .map(|env| matches!(env, "1" | "true"))
        .unwrap_or_default();

    let mut version = env!("CARGO_PKG_VERSION").to_owned();
    if !doing_release {
        version.push('+');
        version.push_str(&git_commit_hash());
        if !git_is_clean() {
            version.push_str(".dirty");
        }
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(
        out_dir.join("version.rs"),
        format!("pub const VERSION: &str = {version:?};"),
    )
    .unwrap();
}
