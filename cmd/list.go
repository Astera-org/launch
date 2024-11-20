package cmd

import (
	"fmt"
	"os"
	"slices"
	"strings"
	"time"

	"astera-infra.com/launch"
	"github.com/jedib0t/go-pretty/v6/table"
	"github.com/spf13/cobra"

	batchv1 "k8s.io/api/batch/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

var utcOffset string

func init() {
	rootCmd.AddCommand(listCmd())
	utcOffset = computeUTCOffset()
}

func listCmd() *cobra.Command {
	validArgs := []string{"jobs", "nodes"}
	cmd := &cobra.Command{
		Use:       "list [RESOURCE]",
		Short:     "List cluster resources",
		ValidArgs: validArgs,
		Args: func(cmd *cobra.Command, args []string) error {
			if len(args) > 1 {
				return fmt.Errorf("expected at most 1 argument, got %d", len(args))
			}
			resource := validArgs[0]
			if len(args) > 0 {
				resource = args[0]
			}
			if !slices.Contains(validArgs, resource) {
				return fmt.Errorf("invalid resource: %s. Must be one of: %v", resource, validArgs)
			}
			return nil
		},
		Run: func(cmd *cobra.Command, args []string) {
			resource := validArgs[0]
			if len(args) > 0 {
				resource = args[0]
			}
			var err error
			switch resource {
			case "jobs":
				err = listJobs(launch.ClusterContext(context))
			case "nodes":
				err = listNodes(launch.ClusterContext(context))
			}
			if err != nil {
				fmt.Println(err)
				os.Exit(1)
			}
		},
	}
	return cmd
}

func computeUTCOffset() string {
	_, offset := time.Now().Zone()
	if offset == 0 {
		return "UTC"
	}
	hours := offset / 3600
	minutes := (offset % 3600) / 60
	return fmt.Sprintf("%+03d:%02d", hours, minutes)
}

func listJobs(context launch.ClusterContext) error {
	kubectl := launch.Kubectl{Server: context.ClusterURL()}
	jobs, err := kubectl.Jobs()
	if err != nil {
		return err
	}
	pods, err := kubectl.Pods()
	if err != nil {
		return err
	}
	jobNameToPods := map[string][]*corev1.Pod{}
	for _, pod := range pods {
		// Note: empty job name is fine, we'll just ignore it later.
		jobName := pod.Labels["job-name"]
		jobNameToPods[jobName] = append(jobNameToPods[jobName], &pod)
	}
	tw := table.NewWriter()
	tw.SetOutputMirror(os.Stdout)
	tw.AppendHeader(table.Row{"name", fmt.Sprintf("created (%s)", utcOffset), "Job status", "launched by"})
	for _, job := range jobs {
		tw.AppendRow(table.Row{job.Name, formatTimestamp(job.CreationTimestamp), formatJobStatus(&job, jobNameToPods[job.Name]), determineUser(&job)})
	}
	tw.Render()
	return nil
}

func listNodes(context launch.ClusterContext) error {
	fmt.Println("Listing nodes...")
	return nil
}

func formatTimestamp(timestamp metav1.Time) string {
	return timestamp.Format("2006-01-02 15:04")
}

// determineUser extracts the user who launched the job from its metadata
func determineUser(job *batchv1.Job) string {
	if job == nil {
		return ""
	}
	// Check for machine user first
	if machineUser := getLaunchedByMachineUser(job.ObjectMeta); machineUser != "" {
		return machineUser
	}
	// Then check for tailscale user
	if tailscaleUser := getLaunchedByTailscaleUser(job.ObjectMeta); tailscaleUser != "" {
		return tailscaleUser
	}
	return ""
}

func getLaunchedByMachineUser(meta metav1.ObjectMeta) string {
	if val, exists := meta.Annotations[launch.LaunchedByMachineUserAnnotation]; exists {
		parts := strings.SplitN(val, "@", 2)
		if len(parts) > 0 {
			return parts[0]
		}
	}
	return ""
}

func getLaunchedByTailscaleUser(meta metav1.ObjectMeta) string {
	if val, exists := meta.Annotations[launch.LaunchedByTailscaleUserAnnotation]; exists {
		parts := strings.SplitN(val, "@", 2)
		if len(parts) > 0 {
			return parts[0]
		}
	}
	return ""
}

func formatJobStatus(job *batchv1.Job, pods []*corev1.Pod) string {
	var result strings.Builder

	for _, condition := range job.Status.Conditions {
		if condition.Status == corev1.ConditionTrue {
			if result.Len() > 0 {
				result.WriteString("\n")
			}

			// Add ANSI color
			switch condition.Type {
			case batchv1.JobFailed:
				result.WriteString(text.FgRed.EscapeSeq())
			case batchv1.JobSuspended:
				result.WriteString(text.FgYellow.EscapeSeq())
			}

			result.WriteString(string(condition.Type))

			// Reset color if we added one
			if condition.Type == batchv1.JobFailed || condition.Type == batchv1.JobSuspended {
				result.WriteString(text.Reset.EscapeSeq())
			}

			if condition.Reason != "" {
				result.WriteString(": ")
				result.WriteString(condition.Reason)
			}
		}
	}

	for _, pod := range pods {
		if result.Len() > 0 {
			result.WriteString("\n")
		}

		// Add ANSI color
		switch pod.Status.Phase {
		case corev1.PodPending:
			result.WriteString(text.FgYellow.EscapeSeq())
		case corev1.PodRunning:
			result.WriteString(text.FgGreen.EscapeSeq())
		case corev1.PodFailed, corev1.PodUnknown:
			result.WriteString(text.FgRed.EscapeSeq())
		}

		result.WriteString(pod.Name)
		result.WriteString(": ")
		result.WriteString(string(pod.Status.Phase))

		// Reset color if we added one
		if pod.Status.Phase != corev1.PodSucceeded {
			result.WriteString(text.Reset.EscapeSeq())
		}
	}

	return result.String()
}
