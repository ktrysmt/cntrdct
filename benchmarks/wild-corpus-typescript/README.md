# cntrdct TypeScript wild corpus

Real-world TypeScript corpus for the R-2 TypeScript pilot (REBUILD.md
R-2.e). Files are verbatim extracts from popular, permissively-licensed
npm/GitHub packages. Unlike `benchmarks/corpus/` (the hand-labelled β
corpus), this corpus is unlabelled: every manifest entry has an empty
`expected` array. Its purpose is eval-drift signal — the
`actual_total` count in the self-replication ledger
(`benchmarks/self-replication/v<release>/`) — not precision/recall
scoring against a ground truth.

## Layout

```
wild-corpus-typescript/
├── README.md
├── manifest.jsonl
└── files/
    ├── zod_ZodError.ts
    ├── ky_Ky.ts
    └── ...
```

`manifest.jsonl` follows the `cntrdct-eval` schema with the additive
`source` / `license` / `sha256` provenance fields, identical to the
Rust and Python wild corpora. Lines beginning with `//` are skipped by
the loader.

## Provenance

Each file carries a three-line header (`// Source:`, `// License:`,
`// Note:`) recording the upstream GitHub release tarball it was
extracted from verbatim. The `// Source:` line doubles as the
clone-drift scope key (`scope_from_provenance`), so files from the same
package cluster together for cross-file clone detection.

| Package | Version | License | Upstream |
| --- | --- | --- | --- |
| zod | 3.23.8 | MIT | https://github.com/colinhacks/zod |
| ky | 1.7.2 | MIT | https://github.com/sindresorhus/ky |

Only `language_typescript()`-parseable `.ts` source is included; `.d.ts`
declaration files, `.tsx` (JSX, out of v0 scope per the
`Language::TypeScript` grammar note), tests, and benchmarks are
excluded. All 16 files parse without recovery
(`IrFile.parse_recovered == false`).

## Refreshing

The corpus is a static, checked-in snapshot. To refresh against newer
upstream releases, re-extract the same file set from the pinned tarball
URLs in `manifest.jsonl`, re-prepend the provenance headers, and
recompute the `sha256` fields. There is no automated fetch script for
the TypeScript corpus in v0; the extraction was a one-off documented
here.

## Note on findings

High-quality library code legitimately exhibits few of the
contradictions cntrdct detects (drifted clones, swapped arguments,
doc/throw mismatches, unreachable code). A zero `actual_total` on this
corpus at a given release is an expected, honest result — it is not
evidence the detectors fail to run. Detector applicability to real
TypeScript is demonstrated by the IR converter extracting functions
from every file and by the labelled `benchmarks/corpus/` TypeScript
fixtures, not by manufacturing findings here.
