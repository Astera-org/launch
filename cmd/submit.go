package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(submitCmd())
}

func submitCmd() *cobra.Command {
	var (
		builder       string
		gpus          int
		gpuMem        int
		allowDirty    bool
		allowUnpushed bool
		namePrefix    string
	)

	cmd := &cobra.Command{
		Use:   "submit -- <command>...",
		Short: "Submit work to the cluster",
		Args:  cobra.MinimumNArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Println("Submitting work...")
		},
	}

	cmd.Flags().StringVar(&builder, "builder", "docker", "How to build the image")
	cmd.Flags().IntVar(&gpus, "gpus", 0, "The minimum number of GPUs per worker")
	cmd.Flags().IntVar(&gpuMem, "gpu-mem", 0, "The minimum GPU RAM memory per worker in gibibyte (GiB, 2^30 bytes)")
	cmd.Flags().BoolVar(&allowDirty, "allow-dirty", false, "Allow dirty git state")
	cmd.Flags().BoolVar(&allowUnpushed, "allow-unpushed", false, "Allow unpushed git changes")
	cmd.Flags().StringVar(&namePrefix, "name-prefix", "", "Job name prefix of up to 20 characters, starting with an alphabetic character (a-z) and further consisting of alphanumeric characters (a-z, 0-9) optionally separated by dashes (-)")

	validBuilders := []string{"docker", "kaniko"}
	cmd.RegisterFlagCompletionFunc("builder", func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		return validBuilders, cobra.ShellCompDirectiveDefault
	})

	cmd.PreRunE = func(cmd *cobra.Command, args []string) error {
		validBuilder := false
		for _, b := range validBuilders {
			if builder == b {
				validBuilder = true
				break
			}
		}
		if !validBuilder {
			return fmt.Errorf("invalid builder: %s. Must be one of: %v", builder, validBuilders)
		}

		if namePrefix != "" {
			if len(namePrefix) > 20 {
				return fmt.Errorf("name-prefix must be at most 20 characters")
			}
			if !isValidNamePrefix(namePrefix) {
				return fmt.Errorf("name-prefix must start with a letter and contain only letters, numbers, and dashes")
			}
		}

		return nil
	}

	return cmd
}

func isValidNamePrefix(prefix string) bool {
	if len(prefix) == 0 {
		return true
	}
	// First character must be a letter
	if !(prefix[0] >= 'a' && prefix[0] <= 'z') {
		return false
	}
	// Rest can be letters, numbers, or dashes
	for _, c := range prefix[1:] {
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-') {
			return false
		}
	}
	return true
}
