use std::collections::HashMap;

use serde::Deserialize;

use super::common;

/// [Node](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#node-v1-core)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub metadata: common::NamespacelessResourceMetadata,
    pub spec: NodeSpec,
    pub status: NodeStatus,
}

/// [NodeSpec](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#nodespec-v1-core)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSpec {
    #[serde(default)]
    pub taints: Vec<Taint>,

    #[serde(default)]
    pub unschedulable: Option<bool>,
}

/// [Taint](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#taint-v1-core)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taint {
    pub key: String,
    pub effect: String,
    #[serde(with = "time::serde::rfc3339")]
    pub time_added: time::OffsetDateTime,
}

/// [NodeStatus](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#nodestatus-v1-core)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    /// List of addresses reachable to the node. Queried from cloud provider, if available. More info:
    /// https://kubernetes.io/docs/concepts/nodes/node/#addresses Note: This field is declared as mergeable, but the
    /// merge key is not sufficiently unique, which can cause data corruption when it is merged. Callers should instead
    /// use a full-replacement patch. See https://pr.k8s.io/79391 for an example. Consumers should assume that addresses
    /// can change during the lifetime of a Node. However, there are some exceptions where this may not be possible,
    /// such as Pods that inherit a Node's address in its own status or consumers of the downward API (status.hostIP).
    pub addresses: Vec<NodeAddress>,

    /// Allocatable represents the resources of a node that are available for scheduling. Defaults to Capacity.
    pub allocatable: HashMap<String, String>,

    /// Capacity represents the total resources of a node. More info:
    /// https://kubernetes.io/docs/concepts/storage/persistent-volumes#capacity
    pub capacity: HashMap<String, String>,

    /// Conditions is an array of current observed node conditions. More info:
    /// https://kubernetes.io/docs/concepts/nodes/node/#condition
    pub conditions: Vec<NodeCondition>,

    /// Set of ids/uuids to uniquely identify the node. More info: https://kubernetes.io/docs/concepts/nodes/node/#info
    pub node_info: NodeInfo,
}

/// [NodeCondition](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.30/#nodecondition-v1-core)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCondition {
    /// Last time we got an update on a given condition.
    #[serde(with = "time::serde::rfc3339")]
    pub last_heartbeat_time: time::OffsetDateTime,

    /// Last time the condition transit from one status to another.
    #[serde(with = "time::serde::rfc3339")]
    pub last_transition_time: time::OffsetDateTime,

    /// Human readable message indicating details about last transition.
    pub message: String,

    /// (brief) reason for the condition's last transition.
    pub reason: String,

    /// Status of the condition, one of True, False, Unknown.
    pub status: String,

    /// Type of node condition.
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAddress {
    /// The node address.
    pub address: String,

    /// Node address type, one of Hostname, ExternalIP or InternalIP.
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    /// The Architecture reported by the node
    pub architecture: String,

    /// Boot ID reported by the node.
    pub boot_id: Option<String>,

    /// ContainerRuntime Version reported by the node through runtime remote API (e.g. containerd://1.4.2).
    pub container_runtime_version: String,

    /// Kernel Version reported by the node from 'uname -r' (e.g. 3.16.0-0.bpo.4-amd64).
    pub kernel_version: String,

    /// KubeProxy Version reported by the node.
    pub kube_proxy_version: String,

    /// Kubelet Version reported by the node.
    pub kubelet_version: String,

    /// MachineID reported by the node. For unique machine identification in the cluster this field is preferred. Learn
    /// more from man(5) machine-id: http://man7.org/linux/man-pages/man5/machine-id.5.html
    pub machine_id: Option<String>,

    /// The Operating System reported by the node
    pub operating_system: String,

    /// OS Image reported by the node from /etc/os-release (e.g. Debian GNU/Linux 7 (wheezy)).
    pub os_image: String,

    /// SystemUUID reported by the node. For unique machine identification MachineID is preferred. This field is
    /// specific to Red Hat hosts
    /// https://access.redhat.com/documentation/en-us/red_hat_subscription_management/1/html/rhsm/uuid
    pub system_uuid: Option<String>,
}
