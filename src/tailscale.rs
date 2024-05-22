use std::collections::HashMap;

use crate::{
    as_ref,
    kubectl::{self, kubectl_use_context},
    process,
};

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

/// Will attempt to restore the kubernetes context on drop.
#[must_use]
pub struct RestoreKubernetesContext(String);

impl Drop for RestoreKubernetesContext {
    fn drop(&mut self) {
        log::debug!("kubernetes context restored to {:?}", &self.0);
        if let Err(error) = kubectl_use_context(&self.0) {
            log::warn!("Failed to restore kubernetes context {:?}: {error}", self.0);
        }
    }
}

pub fn tailscale_configure_kubeconfig(
    tailscale_operator: &str,
) -> Result<RestoreKubernetesContext, Box<dyn std::error::Error>> {
    let previous_context = kubectl::kubectl_current_context()?;
    tailscale()
        .args(as_ref!["configure", "kubeconfig", tailscale_operator])
        .output()?;
    if log::log_enabled!(log::Level::Info) {
        let current_context = kubectl::kubectl_current_context()?;
        if previous_context != current_context {
            log::debug!(
                "kubernetes context switched from {previous_context:?} to {current_context:?}"
            );
        }
    }
    Ok(RestoreKubernetesContext(previous_context))
}
