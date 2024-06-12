use std::{convert::Infallible, fmt, str::FromStr};

#[derive(Debug)]
/// The parsed object representation of `"{user}@{host}"` where `@{host}` is optional. See [`UserHostRef`] for
/// the borrowed version.
pub struct UserHost {
    user: String,
    host: Option<String>,
}

impl UserHost {
    pub fn new(user: String, host: Option<String>) -> Self {
        Self { user, host }
    }

    pub fn parse(value: &str) -> Self {
        UserHostRef::parse(value).to_owned()
    }

    #[inline]
    pub fn user(&self) -> &str {
        &self.user
    }

    #[inline]
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn to_ref(&self) -> UserHostRef<'_> {
        UserHostRef::new(self.user(), self.host())
    }
}

impl FromStr for UserHost {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(value))
    }
}

impl fmt::Display for UserHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_ref().fmt(f)
    }
}

/// See [`UserHost`] for the owned version.
#[derive(Debug)]
pub struct UserHostRef<'a> {
    user: &'a str,
    host: Option<&'a str>,
}

impl<'a> UserHostRef<'a> {
    pub fn new(user: &'a str, host: Option<&'a str>) -> Self {
        Self { user, host }
    }

    pub fn from_user(user: &'a str) -> Self {
        Self::new(user, None)
    }

    pub fn from_user_and_host(user: &'a str, host: &'a str) -> Self {
        Self::new(user, Some(host))
    }

    pub fn parse(value: &'a str) -> Self {
        match value.split_once('@') {
            Some((user, host)) => Self::from_user_and_host(user, host),
            None => Self::from_user(value),
        }
    }

    #[inline]
    pub fn user(&self) -> &'a str {
        self.user
    }

    #[inline]
    pub fn host(&self) -> Option<&'a str> {
        self.host
    }

    pub fn to_owned(&self) -> UserHost {
        UserHost::new(self.user.to_string(), self.host.map(str::to_string))
    }
}

impl<'a> fmt::Display for UserHostRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let user = self.user();
        match self.host() {
            Some(host) => write!(f, "{user}@{host}"),
            None => write!(f, "{user}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! user_host_test {
        ($input:expr, $user:expr, $host:expr) => {
            user_host_test!(@ UserHost, $input, $user, $host);
            user_host_test!(@ UserHostRef, $input, $user, $host);
        };
        (@ $T:ty, $input:expr, $user:expr, $host:expr) => {
            let value = <$T>::parse($input);
            assert_eq!(value.user(), $user);
            assert_eq!(value.host(), $host);
            assert_eq!(value.to_string(), $input);
        };
    }

    #[test]
    fn with_host() {
        user_host_test!("mick@astera.org", "mick", Some("astera.org"));
    }

    #[test]
    fn without_host() {
        user_host_test!("mick", "mick", None);
    }
}
