//go:build unix && arm64 && !unix

// build_tag_interaction_go_007: synthetic corpus fixture (authored, MIT).
// The constraint above requires a build tag and its negation, so this file
// can never be selected for compilation under any GOOS/GOARCH configuration.
package buildtag

func placeholder007() int { return 7 }
