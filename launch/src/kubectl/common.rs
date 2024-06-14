use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GetResource<T> {
    #[serde(rename = "items")]
    pub items: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md#metadata
pub struct ResourceMetadata {
    pub name: String,

    pub namespace: String,

    #[serde(with = "time::serde::rfc3339")]
    pub creation_timestamp: time::OffsetDateTime,

    #[serde(default)]
    pub labels: HashMap<String, String>,

    #[serde(default)]
    pub annotations: HashMap<String, String>,

    #[serde(default)]
    pub owner_references: Vec<OwnerReference>,

    #[serde(default)]
    pub finalizers: Vec<String>,

    #[serde(default)]
    pub generate_name: Option<String>,

    #[serde(default)]
    pub generation: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct OwnerReference {
    #[serde(rename = "name")]
    pub name: String,
}
