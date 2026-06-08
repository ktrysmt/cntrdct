//go:build cgo && !cgo && amd64

// build_tag_interaction_go_005: synthetic corpus fixture (authored, MIT).
// The constraint above requires a build tag and its negation, so this file
// can never be selected for compilation under any GOOS/GOARCH configuration.
package buildtag

func placeholder005() int { return 5 }
