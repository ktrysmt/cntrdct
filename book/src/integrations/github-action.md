# GitHub Action

The reusable GitHub Action (T2-9) consumes the pre-built `cntrdct`
binary and surfaces findings as PR comments matching the GitHub
Annotations convention.

A working sample workflow lives at
[`examples/github-action-usage.yml`](https://github.com/ktrysmt/cntrdct/blob/master/examples/github-action-usage.yml).
The mixed Rust + Python SARIF path is verified end-to-end by
`tests/multilang_config.rs` so the action emits a single combined
SARIF document for repositories that span both languages.

## paths filter

The `paths:` input accepts `<path>:<lang_csv>` per-line entries, e.g.

```yaml
paths: |
  src/:rust
  scripts/:python
  tests/:rust,python
```

`prepare_config.py`, `merge_json.py`, and `merge_sarif.py` (shipped
under `examples/`) implement the filter and the multi-language merge.
