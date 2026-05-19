# eval

`cntrdct eval <CORPUS_DIR>` reports precision, recall, and F1 on a
labelled corpus. The corpus directory must contain a
`manifest.jsonl` file (override with `--manifest <PATH>`) listing the
expected findings.

```sh
cntrdct eval benchmarks/wild-corpus-python
```

## Baseline comparison (Q-15)

```sh
cntrdct eval benchmarks/audit-corpus \
    --baseline sourcerercc \
    --baselines-out baseline-comparison.json
```

The `--baseline` flag runs cntrdct alongside a pinned external
comparator (SourcererCC for `clone-drift`, PyBugLab for `arg-swap`)
and reports side-by-side metrics. Each baseline ships as a
digest-pinned Docker image (`docker run --network=none --rm
--read-only`) so the comparison is reproducible from a clean
environment. `--baselines-skip-run` reuses a previously-cached
baseline JSONL under
`tests/fixtures/baselines/baselines/v<release>/`.

Spec:
[`docs/spec/sota-baselines-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/sota-baselines-v0.md).
