package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var (
	context string
	rootCmd = &cobra.Command{
		Use:     "launch",
		Short:   "A tool to manage work on clusters",
		Version: "0.1.0",
	}
)

func init() {
	// Add global context flag
	rootCmd.PersistentFlags().StringVar(
		&context,
		"context",
		"berkeley",
		"Context (AKA cluster) to use for the operation",
	)

	// Register the valid contexts
	validContexts := []string{"berkeley", "staging", "voltage-park"}
	rootCmd.RegisterFlagCompletionFunc(
		"context",
		func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			return validContexts, cobra.ShellCompDirectiveDefault
		},
	)

	// Add validation for context flag
	rootCmd.PersistentPreRunE = func(cmd *cobra.Command, args []string) error {
		for _, validContext := range validContexts {
			if context == validContext {
				return nil
			}
		}
		return fmt.Errorf("invalid context: %s. Must be one of: %v", context, validContexts)
	}
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
