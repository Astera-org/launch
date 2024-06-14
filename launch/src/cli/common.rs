use log::warn;

use crate::{
    kubectl, tailscale,
    user_host::{UserHost, UserHostRef},
};

pub fn machine_user_host() -> UserHost {
    UserHost::new(
        whoami::username(),
        whoami::fallible::hostname()
            .inspect_err(|error| {
                warn!("Unable to determine hostname: {error}");
            })
            .ok(),
    )
}

pub fn tailscale_user_host() -> Option<UserHost> {
    tailscale::get_login_name()
        .inspect_err(|error| {
            warn!("Unable to determine tailscale user: {error}");
        })
        .ok()
        .as_deref()
        .map(UserHost::parse)
}

pub fn launched_by_machine_user(meta: &kubectl::ResourceMetadata) -> Option<UserHostRef<'_>> {
    meta.annotations
        .get(kubectl::annotation::LAUNCHED_BY_MACHINE_USER)
        .map(|value| UserHostRef::parse(value))
}

pub fn launched_by_tailscale_user(meta: &kubectl::ResourceMetadata) -> Option<UserHostRef<'_>> {
    meta.annotations
        .get(kubectl::annotation::LAUNCHED_BY_TAILSCALE_USER)
        .map(|value| UserHostRef::parse(value))
}
