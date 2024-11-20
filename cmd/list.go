package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func listCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "list [RESOURCE]",
		Short: "List cluster resources",
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Println("Listing works...")
		},
	}
	return cmd
}

func init() {
	rootCmd.AddCommand(listCmd())
}
