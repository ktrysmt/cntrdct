//go:build go1.18 && !go1.18

// build_tag_interaction_go_001: synthetic corpus fixture (authored, MIT).
// The constraint above requires a build tag and its negation, so this file
// can never be selected for compilation under any GOOS/GOARCH configuration.
package buildtag

func placeholder001() int { return 1 }
