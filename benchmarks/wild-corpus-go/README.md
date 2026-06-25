# cntrdct Go wild corpus

Real-world Go corpus for the R-3 Go pilot. Files are
verbatim extracts from popular, permissively-licensed GitHub packages.
Unlike `benchmarks/corpus/` (the hand-labelled β corpus), this corpus is
unlabelled: every manifest entry has an empty `expected` array. Its
purpose is eval-drift signal — the `actual_total` count in the
self-replication ledger (`benchmarks/self-replication/v<release>/`) —
not precision/recall scoring against a ground truth.

## Layout

```
wild-corpus-go/
├── README.md
├── manifest.jsonl
└── files/
    ├── uuid_uuid.go
    ├── logrus_entry.go
    └── ...
```

`manifest.jsonl` follows the `cntrdct-eval` schema with the additive
`source` / `license` / `sha256` provenance fields, identical to the
Rust, Python, and TypeScript wild corpora. Lines beginning with `//`
are skipped by the loader.

## Provenance

Each file carries a three-line header (`// Source:`, `// License:`,
`// Note:`) recording the upstream GitHub release tarball it was
extracted from verbatim. The `// Source:` line doubles as the
clone-drift scope key (`scope_from_provenance`), so files from the same
package cluster together for cross-file clone detection.

| Package | Version | License | Upstream |
| --- | --- | --- | --- |
| google/uuid | 1.6.0 | BSD-3-Clause | https://github.com/google/uuid |
| sirupsen/logrus | 1.9.3 | MIT | https://github.com/sirupsen/logrus |

Only `language()`-parseable top-level `.go` source is included; test
files (`*_test.go`), build-tag-gated platform shims (`terminal_check_*`,
`node_js.go`), and package-doc-only files are excluded. All 16 files
parse without recovery (`IrFile.parse_recovered == false`) and the IR
converter extracts at least one function from every file.

## Refreshing

The corpus is a static, checked-in snapshot. To refresh against newer
upstream releases, re-extract the same file set from the pinned tarball
URLs in `manifest.jsonl`, re-prepend the provenance headers, and
recompute the `sha256` fields (the `sha256` is over the headered file as
checked in). There is no automated fetch script for the Go corpus in v0;
the extraction was a one-off documented here.

## Note on findings

High-quality library code legitimately exhibits few of the
contradictions cntrdct detects (drifted clones, swapped arguments,
doc/panic mismatches, unreachable code). A zero `actual_total` on this
corpus at a given release is an expected, honest result — it is not
evidence the detectors fail to run. Detector applicability to real Go is
demonstrated by the IR converter extracting functions from every file
and by the labelled `benchmarks/corpus/` Go fixtures (the
`pr_miner_go_*` set), not by manufacturing findings here.
