#[derive(Debug, Clone)]
pub struct ContainerImage<'a> {
    pub registry: &'a str,
    pub name: &'a str,
    pub tag: &'a str,
    pub digest: Option<String>,
}

impl<'a> ContainerImage<'a> {
    pub fn new(registry: &'a str, name: &'a str, tag: &'a str) -> Self {
        ContainerImage {
            registry,
            name,
            tag,
            digest: None,
        }
    }
    pub fn image_url(&self) -> String {
        let mut url = format!(
            "{host}/{name}:{tag}",
            host = self.registry,
            name = self.name,
            tag = self.tag
        );

        if let Some(digest) = self.digest.to_owned() {
            url = format!("{url}@{digest}");
        }

        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_without_digest() {
        let image = ContainerImage::new("registry.io", "my-image", "latest");
        assert_eq!(image.image_url(), "registry.io/my-image:latest");
    }

    #[test]
    fn test_image_with_digest() {
        let mut image = ContainerImage::new("registry.io", "my-image", "latest");
        image.digest = Some(String::from("sha256:12345"));
        assert_eq!(
            image.image_url(),
            "registry.io/my-image:latest@sha256:12345"
        );
    }
}
