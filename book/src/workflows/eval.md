# eval

`cntrdct eval <CORPUS_DIR>` runs the full scan pipeline against a
labelled corpus and reports precision, recall, and F1 — both per
detector and overall. It is the routine measurement workflow for
"did detector quality move when I changed the code?".

## Synopsis

```sh
cntrdct eval benchmarks/wild-corpus-python              # JSON to stdout
cntrdct eval benchmarks/audit-corpus                    # JSON to stdout
```

| Flag | Default | Effect |
|---|---|---|
| `--manifest <PATH>` | `<corpus>/manifest.jsonl` | Override the labelled manifest. |

## Manifest shape

Each row of `manifest.jsonl` declares one source file and the
findings expected from it:

```jsonl
{"path": "files/quiet_a.rs", "expected": [
  {"detector_id": "clone-drift", "start_line": 42},
  {"detector_id": "comment-code", "start_line": 87}
]}
```

`ManifestEntry` carries optional `source`, `license`, and `sha256`
fields (M-4) so wild-corpus rows can pin the upstream provenance.

## Self-replication ledger

cntrdct tracks its own precision / recall / F1 across releases rather
than comparing against external tools. The retired Q-15 "SOTA
baseline comparator" (`--baseline` against pinned PyBugLab /
SourcererCC Docker images) was removed in v0.6.0: those projects do
not distribute pre-trained weights or comparison infrastructure in an
installable form, so a reproducible external comparison was
unrealisable.

The replacement is a per-release eval snapshot committed under
`benchmarks/self-replication/v<release>/`:

```sh
cntrdct eval benchmarks/audit-corpus > \
    benchmarks/self-replication/v<release>/cntrdct.jsonl
```

The ledger is refreshed manually per release and carries no CI gate.
A release reviewer reads the per-detector F1 / precision / recall
delta against the previous tag's snapshot to confirm a change did not
regress detection quality.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Eval completed; report written. |
| 1 | Invalid arguments, missing corpus / manifest. |

## See also

- [calibrate](./calibrate.md) — `--audit-recall` mode also operates
  on a corpus directory and is a complementary measurement axis
  (recall against externally-sourced ground truth).
- [scan](./scan.md) — eval reuses the scan pipeline internally; any
  change that affects scan output reflects in eval numbers.
- Spec:
  [`docs/spec/eval-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/eval-v0.md).
