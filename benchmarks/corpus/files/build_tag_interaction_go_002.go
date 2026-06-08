//go:build linux && !linux

// build_tag_interaction_go_002: synthetic corpus fixture (authored, MIT).
// The constraint above requires a build tag and its negation, so this file
// can never be selected for compilation under any GOOS/GOARCH configuration.
package buildtag

func placeholder002() int { return 2 }
