---
name: Bug report
about: Report incorrect detector behaviour, a crash, or any other defect
title: 'bug: '
labels: bug
assignees: ''
---

## What happened

<!-- One or two sentences. What did cntrdct do that was wrong? -->

## Expected behaviour

<!-- What should it have done instead? If a detector fired a false positive,
say which detector and why the flagged construct is correct. If it
missed a true positive, describe the contradiction it should have
caught. -->

## Reproduction

Minimum source that triggers the bug. Inline if short, otherwise link
to a gist or a public commit.

```rust
// or .py — paste the smallest snippet that reproduces the issue
```

Command used:

```sh
cntrdct scan ...
```

## Output

```
<paste the relevant cntrdct output, including the finding(s) in question>
```

## Environment

- cntrdct version: <!-- output of `cntrdct --version` -->
- Install method: <!-- cargo install / pre-built binary / from source -->
- OS and architecture: <!-- e.g. macOS 14 aarch64, Ubuntu 22.04 x86_64 -->
- Rust toolchain (if built from source): <!-- output of `rustc -V` -->

## Additional context

<!-- Links to related issues, recent commits that introduced the
regression, or anything else that helps triage. Optional. -->
