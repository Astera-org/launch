package launch

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"sort"

	corev1 "k8s.io/api/core/v1"

	batchv1 "k8s.io/api/batch/v1"
)

const (
	Namespace                         = "launch"
	LaunchedByMachineUserAnnotation   = "launch.astera.org/launched-by-machine-user"
	LaunchedByTailscaleUserAnnotation = "launch.astera.org/launched-by-tailscale-user"
	VersionAnnotation                 = "launch.astera.org/version"
	kubectlExe                        = "kubectl"
)

type Kubectl struct {
	Server string
}

func (k *Kubectl) Jobs() ([]batchv1.Job, error) {
	cmdArgs := append(k.argsPrefix(), "get", "jobs", "--namespace", Namespace, "--output", "json")
	cmd := exec.Command(kubectlExe, cmdArgs...)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("kubectl error: %v: %s", err, output)
	}

	var jobList batchv1.JobList
	if err := json.Unmarshal(output, &jobList); err != nil {
		return nil, err
	}

	// Sort jobs by creationTimestamp and name
	sort.Slice(jobList.Items, func(i, j int) bool {
		if !jobList.Items[i].CreationTimestamp.Equal(&jobList.Items[j].CreationTimestamp) {
			return jobList.Items[i].CreationTimestamp.Before(&jobList.Items[j].CreationTimestamp)
		}
		return jobList.Items[i].Name < jobList.Items[j].Name
	})

	return jobList.Items, nil
}

func (k *Kubectl) Pods() ([]corev1.Pod, error) {
	cmdArgs := append(k.argsPrefix(), "get", "pods", "--namespace", Namespace, "--output", "json")
	cmd := exec.Command(kubectlExe, cmdArgs...)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("kubectl error: %v: %s", err, output)
	}

	var podList corev1.PodList
	if err := json.Unmarshal(output, &podList); err != nil {
		return nil, err
	}

	// Sort pods by creationTimestamp and name
	sort.Slice(podList.Items, func(i, j int) bool {
		if !podList.Items[i].CreationTimestamp.Equal(&podList.Items[j].CreationTimestamp) {
			return podList.Items[i].CreationTimestamp.Before(&podList.Items[j].CreationTimestamp)
		}
		return podList.Items[i].Name < podList.Items[j].Name
	})

	return podList.Items, nil
}

func (k *Kubectl) argsPrefix() []string {
	return []string{
		"--server", k.Server,
		"--token", "unused",
		// Despite passing `--server` and `--token`, kubectl will still load the
		// kubeconfig if present. By setting `--kubeconfig` to an empty file, we
		// can make sure no other options apply.
		"--kubeconfig", os.DevNull,
	}
}

func (k *Kubectl) Nodes() ([]corev1.Node, error) {
	cmdArgs := append(k.argsPrefix(), "get", "nodes", "--output", "json")
	cmd := exec.Command(kubectlExe, cmdArgs...)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("kubectl error: %v: %s", err, output)
	}

	var nodeList corev1.NodeList
	if err := json.Unmarshal(output, &nodeList); err != nil {
		return nil, err
	}

	return nodeList.Items, nil
}
