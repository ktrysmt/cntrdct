# eval

`cntrdct eval <CORPUS_DIR>` runs the full scan pipeline against a
labelled corpus and reports precision, recall, and F1 — both per
detector and overall. It is the routine measurement workflow for
"did detector quality move when I changed the code?".

## Synopsis

```sh
cntrdct eval benchmarks/wild-corpus-python              # JSON to stdout
cntrdct eval benchmarks/audit-corpus \
    --baseline sourcerercc,pybuglab \
    --baselines-out baseline-comparison.json
```

| Flag | Default | Effect |
|---|---|---|
| `--manifest <PATH>` | `<corpus>/manifest.jsonl` | Override the labelled manifest. |
| `--baseline <NAMES>` | unset | Q-15: comma-separated baseline names (currently `sourcerercc`, `pybuglab`). Triggers side-by-side comparison. |
| `--baselines-out <PATH>` | stdout | Write the `BaselineComparisonReport` JSON to disk. Without this flag the report follows the `EvalReport` on stdout. |
| `--baselines-skip-run` | off | Read pre-cached baseline JSONL under `<corpus>/../baselines/v<release>/<name>.jsonl` instead of invoking Docker. |

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

## Baseline comparison (Q-15)

The `--baseline` flag runs cntrdct alongside a pinned external
comparator and reports side-by-side precision / recall / F1 on the
same corpus. Baselines ship as digest-pinned Docker images invoked
with `docker run --network=none --rm --read-only` so the comparison
is reproducible from a clean environment.

| Baseline | Detector | Image |
|---|---|---|
| `sourcerercc` | `clone-drift` | `baselines/sourcerercc/Dockerfile` |
| `pybuglab` | `arg-swap` (Python) | `baselines/pybuglab/Dockerfile` |

The release-time path is:

1. Run Docker locally on the maintainer's workstation against the
   audit + wild corpora.
2. Commit the resulting JSONL under
   `benchmarks/baselines/v<release>/<name>.jsonl`.
3. Re-run with `--baselines-skip-run` to regenerate the report from
   the committed artefact in CI.

`BaselineComparisonReport.priors_default_sha256` records the SHA-256
of the priors file the comparison ran against so the numbers can be
re-derived from a known-good binary state.

Phase D — live Docker runs against real corpora and populated
README numbers — is pending on the maintainer workstation as of
v0.4.3.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Eval completed; report written. |
| 1 | Invalid arguments, missing corpus / manifest, baseline misconfig. |

## See also

- [calibrate](./calibrate.md) — `--audit-recall` mode also operates
  on a corpus directory and is a complementary measurement axis
  (recall against externally-sourced ground truth).
- [scan](./scan.md) — eval reuses the scan pipeline internally; any
  change that affects scan output reflects in eval numbers.
- Spec:
  [`docs/spec/eval-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/eval-v0.md),
  [`docs/spec/sota-baselines-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/sota-baselines-v0.md).
