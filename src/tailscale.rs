use std::collections::HashMap;

use crate::{as_ref, process};

#[cfg(target_os = "macos")]
fn tailscale() -> process::Command {
    // There are multiple installation methods for Tailscale on Mac. Not all methods place the
    // `tailscale` binary in the path. See https://github.com/tailscale/tailscale/issues/2553.
    use std::sync::OnceLock;

    fn can_run(program: &str) -> bool {
        std::process::Command::new(program)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    static PROGRAM: OnceLock<&'static str> = OnceLock::new();
    let program = PROGRAM.get_or_init(|| {
        if can_run("tailscale") {
            "tailscale"
        } else {
            "/Applications/Tailscale.app/Contents/MacOS/Tailscale"
        }
    });

    process::Command::new(program)
}

#[cfg(not(target_os = "macos"))]
fn tailscale() -> process::Command {
    process::Command::new("tailscale")
}

#[derive(serde::Deserialize)]
struct TailscaleStatusRoot {
    #[serde(rename = "Self")]
    me: TailscaleStatusSelf,

    #[serde(rename = "User")]
    users: Option<HashMap<String, TailscaleStatusUser>>,
}

#[derive(serde::Deserialize)]
struct TailscaleStatusSelf {
    #[serde(rename = "UserID")]
    user_id: i64,
}

#[derive(serde::Deserialize)]
struct TailscaleStatusUser {
    #[serde(rename = "LoginName")]
    login_name: String,
}

pub fn tailscale_get_user() -> Result<String, Box<dyn std::error::Error>> {
    let output = tailscale().args(as_ref!["status", "--json",]).output()?;

    let json: TailscaleStatusRoot = serde_json::from_slice(&output.stdout)?;

    let Some(users) = json.users else {
        return Err("Unable to determine tailscale user, are you logged in?".into());
    };

    Ok(users
        .get(&json.me.user_id.to_string())
        .expect("failed to obtain tailscale user name")
        .login_name
        .clone())
}
