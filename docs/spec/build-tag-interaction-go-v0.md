# build-tag-interaction-go v0

Status: implemented 2026-06-08 (Go language-specific detector). Spec
mirrors the `config-interaction` contract (`config-interaction-v0.md`)
for the Go `//go:build` mechanism.

## Scope

Flag a Go source file whose modern `//go:build` constraint is
*unsatisfiable*: it requires a build tag both positively and negatively
in the same conjunction (e.g. `//go:build linux && !linux`). Such a file
never builds for any `GOOS`/`GOARCH`/tag configuration — the Go analogue
of the Rust `config-interaction` `cfg(all(X, not(X)))` contradiction
(dead-block / inconsistent-feature anomaly class, `tartler-eurosys-2011`).

Language-specific detector under the two-tier layout
(`src/detectors/lang/go_build_tag_interaction.rs`), Go-only, Pattern B
(reads source directly per ir-v0.md §F5 — the constraint lives in leading
comments, so no AST walk is required).

## F1 — Input

`&[IrFile]` filtered to `Language::Go`. The detector reads
`IrFile.source` line by line; `parse_recovered` files are skipped.

## F2 — Constraint location

Only the modern `//go:build` line is analysed. Per the Go toolchain it
must appear before the `package` clause, preceded only by blank lines and
other line comments. The scan walks leading lines, stops at the first
`package` line, and processes the first `//go:build ` line found (Go
permits exactly one). A `//go:build`-looking string after the package
clause is never read.

## F3 — Contradiction (decidable subset)

The expression grammar is build tags (`[A-Za-z0-9_.]+`), `!`, `&&`,
`||`, and parentheses. v0 decides only the pure-conjunction subset:

- if the expression contains `||` (disjunction), it is INDETERMINATE
  (the single-conjunction reasoning is unsound) — never flagged;
- if a `!` immediately precedes a parenthesis (`!( … )`, a De Morgan
  disjunction), INDETERMINATE — never flagged;
- any character outside the grammar makes the whole expression
  INDETERMINATE — never flagged (precision-first).

Within the subset, each tag occurrence is classified positive or
negative by the parity of the immediately-preceding `!` run (`!!x` is
positive). If any tag appears in BOTH the positive and negative sets the
constraint is always false; the lexicographically-first such tag is
reported.

## F4 — Finding shape

- `detector_id = "build-tag-interaction-go"`
- `primary` = the `//go:build` line (1-based, full line span)
- `raw_severity = Warning`, `anomaly_class = Logic`
- `evidence.raw.kind = "go-build-tag-contradiction"` with `constraint`
  and `conflicting_tag`
- `evidence.citation_keys = ["tartler-eurosys-2011", "nadi-icse-2014"]`
- `language_citation_status = Unconfirmed` (no Go-subject peer-reviewed
  grounding; see survey)

## Citation (P1)

Concept-grounded by `tartler-eurosys-2011` (dead-block / inconsistent
feature) and `nadi-icse-2014` (contradictory configuration constraints
recur in production code). Both study C / Linux KConfig; Go coverage is
`Unconfirmed` per `citations-policy.md`. Survey:
`docs/surveys/build-tag-interaction-go-2026-06.md`.

## Non-goals (v0)

- Legacy `// +build` constraints, and `//go:build` / `// +build`
  mismatch (handled by `go vet`).
- Disjunctive unsatisfiability (e.g. `(linux && !linux) || (a && !a)`)
  and De Morgan forms — would require a SAT-style evaluation; out of the
  precision-first decidable subset.
- Cross-file or cross-tag semantic conflicts (e.g. a tag that no real
  configuration ever sets); requires external configuration knowledge.

## Test plan

| Case | Input | Expectation |
| --- | --- | --- |
| simple | `//go:build linux && !linux` | 1 Finding at the constraint line |
| nested | `//go:build (linux && amd64) && !linux` | 1 Finding |
| satisfiable | `//go:build linux && amd64` | 0 Findings |
| disjunction | `//go:build linux \|\| !linux` | 0 Findings (indeterminate) |
| de morgan | `//go:build linux && !(linux && amd64)` | 0 Findings |
| double-neg | `//go:build !!linux && !linux` | 1 Finding |
| non-go | the same text in a `.rs` file | 0 Findings (Go-only) |

Corpus: 8 positives `benchmarks/corpus/files/build_tag_interaction_go_0{01..08}.go`
(meets the `corpus_shape` ≥8-positives-per-registered-detector contract)
+ matching `benchmarks/labelled-findings.jsonl` TruePositive lines;
`benchmarks/priors-default.json` carries an additive
`build-tag-interaction-go` prior (tp=8/fp=0/jeffreys).
