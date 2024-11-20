package launch

// ClusterContext represents different cluster environments
type ClusterContext string

const (
	Berkeley    ClusterContext = "berkeley"
	Staging     ClusterContext = "staging"
	VoltagePark ClusterContext = "voltage-park"
)

// ClusterURL returns the cluster URL for the given context
func (c ClusterContext) ClusterURL() string {
	return "https://" + string(c) + "-tailscale-operator.taila1eba.ts.net"
}

// HeadlampURL returns the headlamp URL for the given context
func (c ClusterContext) HeadlampURL() string {
	return "https://" + string(c) + "-headlamp.taila1eba.ts.net"
}

// DockerHost returns the docker host for the given context
func (c ClusterContext) DockerHost() string {
	return string(c) + "-docker.taila1eba.ts.net"
}

// DockerHostInsideCluster returns the docker host inside the cluster
func (c *ClusterContext) DockerHostInsideCluster() string {
	// Configured in `k8s-cluster.yml` under `containerd_registries_mirrors`.
	return "astera-infra.com"
}
