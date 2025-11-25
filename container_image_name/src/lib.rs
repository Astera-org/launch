//! Container image name types based on [reference.go](https://github.com/distribution/distribution/blob/v2.7.1/reference/reference.go):
//!
//! ```go
//! // Package reference provides a general type to represent any way of referencing images within the registry.
//! // Its main purpose is to abstract tags and digests (content-addressable hash).
//! //
//! // Grammar
//! //
//! // reference                       := name [ ":" tag ] [ "@" digest ]
//! // name                            := [domain '/'] path-component ['/' path-component]*
//! // domain                          := domain-component ['.' domain-component]* [':' port-number]
//! // domain-component                := /([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9])/
//! // port-number                     := /[0-9]+/
//! // path-component                  := alpha-numeric [separator alpha-numeric]*
//! // alpha-numeric                   := /[a-z0-9]+/
//! // separator                       := /[_.]|__|[-]*/
//! //
//! // tag                             := /[\w][\w.-]{0,127}/
//! //
//! // digest                          := digest-algorithm ":" digest-hex
//! // digest-algorithm                := digest-algorithm-component [ digest-algorithm-separator digest-algorithm-component ]*
//! // digest-algorithm-separator      := /[+.-_]/
//! // digest-algorithm-component      := /[A-Za-z][A-Za-z0-9]*/
//! // digest-hex                      := /[0-9a-fA-F]{32,}/ ; At least 128 bit digest value
//! //
//! // identifier                      := /[a-f0-9]{64}/
//! // short-identifier                := /[a-f0-9]{6,64}/
//! ```

use std::{borrow::Cow, ops::Range, str::FromStr, sync::LazyLock};

#[cfg(feature = "serde")]
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidContainerImageNameMarker;

impl std::error::Error for InvalidContainerImageNameMarker {}

impl std::fmt::Display for InvalidContainerImageNameMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid container image name")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidContainerImageName(String);

impl std::error::Error for InvalidContainerImageName {}

impl std::fmt::Display for InvalidContainerImageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid container image name: {:?}", self.0)
    }
}

const REGISTRY_SUFFIX: char = '/';
const PORT_PREFIX: char = ':';
const TAG_PREFIX: char = ':';
const DIGEST_ALGORITHM_PREFIX: char = '@';
const DIGEST_HEX_PREFIX: char = ':';

#[derive(Copy, Clone)]
struct IndicesRegistry {
    // NOTE: domain_start is implicitly 0.
    port_start: Option<usize>,
}

#[derive(Copy, Clone)]
struct IndicesDigest {
    algorithm_start: usize,
    hex_start: usize,
}

#[derive(Copy, Clone)]
struct Indices {
    registry_start: Option<IndicesRegistry>,
    path_start: usize,
    tag_start: Option<usize>,
    digest_start: Option<IndicesDigest>,
}

impl Indices {
    #[inline]
    fn domain_range(&self) -> Option<Range<usize>> {
        self.registry_start.map(|registry_start| {
            0..registry_start
                .port_start
                .map(|x| x.wrapping_sub(PORT_PREFIX.len_utf8()))
                .unwrap_or_else(|| self.path_start.wrapping_sub(REGISTRY_SUFFIX.len_utf8()))
        })
    }

    #[inline]
    fn domain<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.domain_range().map(|range| &buffer[range])
    }

    #[inline]
    fn port_range(&self) -> Option<Range<usize>> {
        self.registry_start
            .and_then(|registry_start| registry_start.port_start)
            .map(|port_start| port_start..self.path_start.wrapping_sub(REGISTRY_SUFFIX.len_utf8()))
    }

    #[inline]
    fn port<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.port_range().map(|range| &buffer[range])
    }

    #[inline]
    fn registry_range(&self) -> Option<Range<usize>> {
        self.registry_start
            .map(|_| 0..self.path_start.wrapping_sub(REGISTRY_SUFFIX.len_utf8()))
    }

    #[inline]
    fn registry<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.registry_range().map(|range| &buffer[range])
    }

    #[inline]
    fn path_range(&self, buffer_len: usize) -> Range<usize> {
        self.path_start
            ..self
                .tag_start
                .map(|x| x.wrapping_sub(TAG_PREFIX.len_utf8()))
                .or(self.digest_start.map(|x| {
                    x.algorithm_start
                        .wrapping_sub(DIGEST_ALGORITHM_PREFIX.len_utf8())
                }))
                .unwrap_or(buffer_len)
    }

    #[inline]
    fn path<'a>(&self, buffer: &'a str) -> &'a str {
        &buffer[self.path_range(buffer.len())]
    }

    #[inline]
    fn tag_range(&self, buffer_len: usize) -> Option<Range<usize>> {
        self.tag_start.map(|tag_start| {
            tag_start
                ..self
                    .digest_start
                    .map(|x| {
                        x.algorithm_start
                            .wrapping_sub(DIGEST_ALGORITHM_PREFIX.len_utf8())
                    })
                    .unwrap_or(buffer_len)
        })
    }

    #[inline]
    fn tag<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.tag_range(buffer.len()).map(|range| &buffer[range])
    }

    #[inline]
    fn digest_algorithm_range(&self) -> Option<Range<usize>> {
        self.digest_start.map(|digest_start| {
            digest_start.algorithm_start
                ..digest_start
                    .hex_start
                    .wrapping_sub(DIGEST_HEX_PREFIX.len_utf8())
        })
    }

    #[inline]
    fn digest_algorithm<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.digest_algorithm_range().map(|range| &buffer[range])
    }

    #[inline]
    fn digest_hex_range(&self, buffer_len: usize) -> Option<Range<usize>> {
        self.digest_start
            .map(|digest_start| digest_start.hex_start..buffer_len)
    }

    #[inline]
    fn digest_hex<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.digest_hex_range(buffer.len())
            .map(|range| &buffer[range])
    }

    #[inline]
    fn digest_range(&self, buffer_len: usize) -> Option<Range<usize>> {
        self.digest_start
            .map(|digest_start| digest_start.algorithm_start..buffer_len)
    }

    #[inline]
    fn digest<'a>(&self, buffer: &'a str) -> Option<&'a str> {
        self.digest_range(buffer.len()).map(|range| &buffer[range])
    }
}

impl FromStr for Indices {
    type Err = InvalidContainerImageNameMarker;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static IMAGE_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(concat!(
                r"^",
                r"(?:(?:(?P<domain>[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?)+)(?::(?P<port>[0-9]+))?)\/)?",
                r"(?P<name>[a-z0-9]+(?:[_.]|__|[-]*[a-z0-9]+)*)(?:\/(?:[a-z0-9]+(?:[_.]|__|[-]*[a-z0-9]+)*))*",
                r"(?::(?P<tag>[\w][\w.-]{0,127}))?",
                r"(?:@(?P<algorithm>[A-Za-z][A-Za-z0-9]*(?:[+.-_][A-Za-z][A-Za-z0-9]*)*):(?P<hex>[0-9a-fA-F]{32,}))?",
                r"$"
            )).unwrap()
        });

        let captures = IMAGE_NAME_REGEX.captures(s).ok_or(InvalidContainerImageNameMarker)?;
        // NOTE: The first sub-capture match, index 0, matches the entire string.
        // NOTE: Obtaining match data by index rather than group name to avoid string lookup.
        Ok(Self {
            registry_start: captures.get(1).map(|m| {
                debug_assert_eq!(m.start(), 0);
                IndicesRegistry {
                    port_start: captures.get(2).map(|m| m.start()),
                }
            }),
            path_start: captures.get(3).map(|m| m.start()).ok_or(InvalidContainerImageNameMarker)?,
            tag_start: captures.get(4).map(|m| m.start()),
            digest_start: captures.get(5).map(|m| IndicesDigest {
                algorithm_start: m.start(),
                hex_start: captures.get(6).unwrap().start(),
            }),
        })
    }
}

macro_rules! impl_image_name_common {
    ($T:ident $(<$lt:tt>)?) => {
        impl$(<$lt>)? $T$(<$lt>)? {
            /// Returns the `<domain>` section of the string documented at [`ImageName`].
            pub fn domain(&self) -> Option<&$($lt)? str> {
                self.indices.domain(&self.buffer)
            }

            /// Returns the `<port>` section of the string documented at [`ImageName`].
            pub fn port(&self) -> Option<&$($lt)? str> {
                self.indices.port(&self.buffer)
            }

            /// Returns the `<domain>(:<port>)?` section of the string documented at [`ImageName`].
            pub fn registry(&self) -> Option<&$($lt)? str> {
                self.indices.registry(&self.buffer)
            }

            /// Returns the `<path>` section of the string documented at [`ImageName`]. This is Returns the only required section.
            pub fn path(&self) -> &$($lt)? str {
                self.indices.path(&self.buffer)
            }

            /// Returns the `<tag>` section of the string documented at [`ImageName`].
            pub fn tag(&self) -> Option<&$($lt)? str> {
                self.indices.tag(&self.buffer)
            }

            /// Returns the `<algorithm>` section of the string documented at [`ImageName`].
            pub fn digest_algorithm(&self) -> Option<&$($lt)? str> {
                self.indices.digest_algorithm(&self.buffer)
            }

            /// Returns the `<hex>` section of the string documented at [`ImageName`].
            pub fn digest_hex(&self) -> Option<&$($lt)? str> {
                self.indices.digest_hex(&self.buffer)
            }

            /// Returns the `<digest>` section of the string documented at [`ImageName`].
            pub fn digest(&self) -> Option<&$($lt)? str> {
                self.indices.digest(&self.buffer)
            }
        }

        impl$(<$lt>)? ::core::cmp::PartialEq for $T$(<$lt>)? {
            fn eq(&self, other: &Self) -> bool {
                self.buffer == other.buffer
            }
        }

        impl$(<$lt>)? ::core::cmp::Eq for $T$(<$lt>)? {}

        impl$(<$lt>)? ::core::cmp::PartialOrd for $T$(<$lt>)? {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl$(<$lt>)? ::core::cmp::Ord for $T$(<$lt>)? {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.buffer.cmp(&other.buffer)
            }
        }

        impl$(<$lt>)? ::core::hash::Hash for $T$(<$lt>)? {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.buffer.hash(state);
            }
        }

        // NOTE: It would make sense for ImageName to deref to ImageNameRef instead, but
        // this can not be done because we can not return a reference to a new object
        // created inside deref. Instead, both just deref to &str.
        impl$(<$lt>)? ::core::ops::Deref for $T$(<$lt>)? {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.buffer
            }
        }

        impl$(<$lt>)? ::std::fmt::Debug for $T$(<$lt>)? {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                (**self).fmt(f)
            }
        }

        impl$(<$lt>)? ::std::fmt::Display for $T$(<$lt>)? {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                (**self).fmt(f)
            }
        }
    };
}

/// Represents a parsed container image name. The container image name is written as:
/// ```txt
/// <domain>:<port>/<path>:<tag>@<algorithm>:<hex>
/// <registry----->/<path>:<tag>@<digest--------->
/// ```
/// Only the `<path>` section is required to be present. The `<port>` section can only be present if
/// the `<domain>` section is present. The `<registry>` section equals the combination of
/// `<domain>:<port>` and the `<digest>` secotion equals the combination of `<algorithm>:<hex>`.
#[derive(Clone)]
pub struct ImageName {
    buffer: String,
    indices: Indices,
}

impl ImageName {
    pub fn new(value: String) -> Result<Self, InvalidContainerImageName> {
        let indices = match value.parse() {
            Ok(indices) => indices,
            Err(InvalidContainerImageNameMarker) => {
                return Err(InvalidContainerImageName(value))
            }
        };
        Ok(Self {
            indices,
            buffer: value,
        })
    }

    pub fn builder<'a>(path: impl Into<Cow<'a, str>>) -> ImageNameBuilder<'a> {
        ImageNameBuilder::new(path)
    }

    pub fn as_builder(&self) -> ImageNameBuilder<'_> {
        let mut builder = ImageNameBuilder::new(self.path());
        if let Some(registry) = self.registry() {
            builder = builder.with_registry(registry);
        }
        if let Some(tag) = self.tag() {
            builder = builder.with_tag(tag);
        }
        if let Some(digest) = self.digest() {
            builder = builder.with_digest(digest);
        }
        builder
    }

    pub fn as_ref(&self) -> ImageNameRef<'_> {
        ImageNameRef {
            buffer: &self.buffer[..],
            indices: self.indices,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.buffer
    }
}

impl_image_name_common!(ImageName);

impl FromStr for ImageName {
    type Err = InvalidContainerImageNameMarker;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ImageNameRef::new(s).map(ImageNameRef::to_owned)
    }
}

impl From<ImageNameRef<'_>> for ImageName {
    fn from(value: ImageNameRef<'_>) -> Self {
        value.to_owned()
    }
}

impl TryFrom<String> for ImageName {
    type Error = InvalidContainerImageName;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImageName> for String {
    fn from(value: ImageName) -> Self {
        value.buffer
    }
}

#[cfg(feature = "serde")]
impl Serialize for ImageName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.buffer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ImageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(Deserialize::deserialize(deserializer)?).map_err(::serde::de::Error::custom)
    }
}

/// A version of [`ImageName`] that only borrows its buffer.
#[derive(Copy, Clone)]
pub struct ImageNameRef<'a> {
    buffer: &'a str,
    indices: Indices,
}

impl<'a> ImageNameRef<'a> {
    pub fn new(value: &'a str) -> Result<Self, InvalidContainerImageNameMarker> {
        Ok(Self {
            buffer: value,
            indices: value.parse()?,
        })
    }

    pub fn as_builder(self) -> ImageNameBuilder<'a> {
        let mut builder = ImageNameBuilder::new(self.path());
        if let Some(registry) = self.registry() {
            builder = builder.with_registry(registry);
        }
        if let Some(tag) = self.tag() {
            builder = builder.with_tag(tag);
        }
        if let Some(digest) = self.digest() {
            builder = builder.with_digest(digest);
        }
        builder
    }

    pub fn to_owned(self) -> ImageName {
        ImageName {
            buffer: self.buffer.to_owned(),
            indices: self.indices,
        }
    }

    pub fn as_str(self) -> &'a str {
        self.buffer
    }
}

impl_image_name_common!(ImageNameRef<'a>);

impl<'a> From<&'a ImageName> for ImageNameRef<'a> {
    fn from(value: &'a ImageName) -> Self {
        value.as_ref()
    }
}

impl<'a> TryFrom<&'a str> for ImageNameRef<'a> {
    type Error = InvalidContainerImageNameMarker;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> From<ImageNameRef<'a>> for &'a str {
    fn from(value: ImageNameRef<'a>) -> Self {
        value.buffer
    }
}

#[cfg(feature = "serde")]
impl Serialize for ImageNameRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.buffer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ImageNameRef<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(Deserialize::deserialize(deserializer)?).map_err(::serde::de::Error::custom)
    }
}

enum ImageNameBuilderRegistry<'a> {
    Registry(Cow<'a, str>),
    DomainPort {
        domain: Cow<'a, str>,
        port: Option<Cow<'a, str>>,
    },
}

impl ImageNameBuilderRegistry<'_> {
    fn len(&self) -> usize {
        (match self {
            ImageNameBuilderRegistry::Registry(registry) => registry.len(),
            ImageNameBuilderRegistry::DomainPort { domain, port } => {
                domain.len()
                    + port
                        .as_ref()
                        .map(|port| PORT_PREFIX.len_utf8() + port.len())
                        .unwrap_or_default()
            }
        }) + REGISTRY_SUFFIX.len_utf8()
    }

    fn write(&self, buffer: &mut String) {
        match self {
            ImageNameBuilderRegistry::Registry(registry) => {
                buffer.push_str(registry);
            }
            ImageNameBuilderRegistry::DomainPort { domain, port } => {
                buffer.push_str(domain);
                buffer.push(PORT_PREFIX);
                if let Some(port) = port {
                    buffer.push_str(port);
                }
            }
        }
        buffer.push(REGISTRY_SUFFIX)
    }
}

enum ImageNameBuilderDigest<'a> {
    Digest(Cow<'a, str>),
    AlgorithmHex {
        algorithm: Cow<'a, str>,
        hex: Cow<'a, str>,
    },
}

impl ImageNameBuilderDigest<'_> {
    fn len(&self) -> usize {
        DIGEST_ALGORITHM_PREFIX.len_utf8()
            + (match self {
                ImageNameBuilderDigest::Digest(digest) => digest.len(),
                ImageNameBuilderDigest::AlgorithmHex { algorithm, hex } => {
                    algorithm.len() + DIGEST_HEX_PREFIX.len_utf8() + hex.len()
                }
            })
    }

    fn write(&self, buffer: &mut String) {
        buffer.push(DIGEST_ALGORITHM_PREFIX);
        match self {
            ImageNameBuilderDigest::Digest(digest) => buffer.push_str(digest),
            ImageNameBuilderDigest::AlgorithmHex { algorithm, hex } => {
                buffer.push_str(algorithm);
                buffer.push(DIGEST_HEX_PREFIX);
                buffer.push_str(hex);
            }
        }
    }
}

pub struct ImageNameBuilder<'a> {
    registry: Option<ImageNameBuilderRegistry<'a>>,
    path: Cow<'a, str>,
    tag: Option<Cow<'a, str>>,
    digest: Option<ImageNameBuilderDigest<'a>>,
}

impl<'a> ImageNameBuilder<'a> {
    fn new(path: impl Into<Cow<'a, str>>) -> Self {
        Self {
            registry: None,
            path: path.into(),
            tag: None,
            digest: None,
        }
    }

    pub fn with_registry(mut self, registry: impl Into<Cow<'a, str>>) -> Self {
        self.registry = Some(ImageNameBuilderRegistry::Registry(registry.into()));
        self
    }

    pub fn with_domain_and_port(
        mut self,
        domain: impl Into<Cow<'a, str>>,
        port: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        self.registry = Some(ImageNameBuilderRegistry::DomainPort {
            domain: domain.into(),
            port: port.into(),
        });
        self
    }

    pub fn with_path(mut self, path: impl Into<Cow<'a, str>>) -> Self {
        self.path = path.into();
        self
    }

    pub fn with_tag(mut self, tag: impl Into<Cow<'a, str>>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn with_digest(mut self, digest: impl Into<Cow<'a, str>>) -> Self {
        self.digest = Some(ImageNameBuilderDigest::Digest(digest.into()));
        self
    }

    pub fn with_algorithm_and_hex(
        mut self,
        algorithm: impl Into<Cow<'a, str>>,
        hex: impl Into<Cow<'a, str>>,
    ) -> Self {
        self.digest = Some(ImageNameBuilderDigest::AlgorithmHex {
            algorithm: algorithm.into(),
            hex: hex.into(),
        });
        self
    }

    pub fn build(self) -> Result<ImageName, InvalidContainerImageName> {
        let mut buffer = String::with_capacity(
            self.registry.as_ref().map(|x| x.len()).unwrap_or_default()
                + self.path.len()
                + self
                    .tag
                    .as_ref()
                    .map(|x| x.len() + TAG_PREFIX.len_utf8())
                    .unwrap_or_default()
                + self.digest.as_ref().map(|x| x.len()).unwrap_or_default(),
        );

        if let Some(registry) = self.registry {
            registry.write(&mut buffer)
        }
        buffer.push_str(&self.path);
        if let Some(tag) = self.tag {
            buffer.push(TAG_PREFIX);
            buffer.push_str(&tag);
        }
        if let Some(digest) = self.digest {
            digest.write(&mut buffer);
        }
        ImageName::new(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_name_parsing_works() {
        {
            let name = ImageNameRef::new("org-name/img-name").unwrap();
            assert_eq!(name.domain(), None);
            assert_eq!(name.port(), None);
            assert_eq!(name.registry(), None);
            assert_eq!(name.path(), "org-name/img-name");
            assert_eq!(name.tag(), None);
            assert_eq!(name.digest(), None);
            assert_eq!(name.digest_algorithm(), None);
            assert_eq!(name.digest_hex(), None);
        }

        {
            let name = ImageNameRef::new("reg.io/org-name/img-name:latest").unwrap();
            assert_eq!(name.domain(), Some("reg.io"));
            assert_eq!(name.port(), None);
            assert_eq!(name.registry(), Some("reg.io"));
            assert_eq!(name.path(), "org-name/img-name");
            assert_eq!(name.tag(), Some("latest"));
            assert_eq!(name.digest(), None);
            assert_eq!(name.digest_algorithm(), None);
            assert_eq!(name.digest_hex(), None);
        }

        {
            let name = ImageNameRef::new("reg.io:12345/org-name/img-name:latest").unwrap();
            assert_eq!(name.domain(), Some("reg.io"));
            assert_eq!(name.port(), Some("12345"));
            assert_eq!(name.registry(), Some("reg.io:12345"));
            assert_eq!(name.path(), "org-name/img-name");
            assert_eq!(name.tag(), Some("latest"));
            assert_eq!(name.digest(), None);
            assert_eq!(name.digest_algorithm(), None);
            assert_eq!(name.digest_hex(), None);
        }

        {
            let name = ImageNameRef::new(
                "reg.io/org-name/img-name@sha256:01234567aaaaaaaa01234567aaaaaaaa",
            )
            .unwrap();
            assert_eq!(name.domain(), Some("reg.io"));
            assert_eq!(name.port(), None);
            assert_eq!(name.registry(), Some("reg.io"));
            assert_eq!(name.path(), "org-name/img-name");
            assert_eq!(name.tag(), None);
            assert_eq!(
                name.digest(),
                Some("sha256:01234567aaaaaaaa01234567aaaaaaaa")
            );
            assert_eq!(name.digest_algorithm(), Some("sha256"));
            assert_eq!(name.digest_hex(), Some("01234567aaaaaaaa01234567aaaaaaaa"));
        }

        {
            assert_eq!(ImageNameRef::new(".").err().unwrap(), InvalidContainerImageNameMarker); // invalid path.
            assert_eq!(
                ImageNameRef::new("a@sha256:1234").err().unwrap(),
                InvalidContainerImageNameMarker,
            ); // digest too short.
        }
    }

    #[test]
    fn image_name_builder_works() {
        {
            assert_eq!(
                ImageNameRef::new("org-name/img-name")
                    .unwrap()
                    .as_builder()
                    .with_algorithm_and_hex("sha256", "12345678aaaaaaaa12345678aaaaaaaa")
                    .build()
                    .unwrap()
                    .as_str(),
                "org-name/img-name@sha256:12345678aaaaaaaa12345678aaaaaaaa"
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn image_name_serde_works() {
        {
            let des = ImageName::new("org-name/img-name".to_string()).unwrap();
            let ser = r#""org-name/img-name""#;
            assert_eq!(serde_json::to_string(&des).unwrap(), ser);
            assert_eq!(serde_json::from_str::<ImageName>(ser).unwrap(), des);
        }

        {
            let des = ImageNameRef::new("org-name/img-name").unwrap();
            let ser = r#""org-name/img-name""#;
            assert_eq!(serde_json::to_string(&des).unwrap(), ser);
            assert_eq!(serde_json::from_str::<ImageNameRef>(ser).unwrap(), des);
        }
    }
}
