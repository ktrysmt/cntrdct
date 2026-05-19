//! cntrdct SOTA baseline-comparator harness — Q-15.
//!
//! Spec: `docs/spec/sota-baselines-v0.md`.
//!
//! Loads normalised JSONL output from an external baseline tool
//! (SourcererCC for `clone-drift`, PyBugLab for `arg-swap`), runs
//! the same eval-v0 §F3 matching rule cntrdct's own `cntrdct::eval`
//! applies,
//! and assembles a [`BaselineComparisonReport`] suitable for
//! hand-transcription into the README's "Latest baseline comparison"
//! section.
//!
//! P3 boundary: the Phase B host-side path opens no sockets. The
//! Docker image is run with `--network=none --read-only` (per spec
//! F4); the host process only invokes `docker run` and reads the
//! resulting JSONL. The fixture-driven test suite skips Docker
//! entirely via the `--baselines-skip-run` path, so the
//! `network-isolation` CI job covers the comparator surface
//! transparently.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::calibration::{compute_lower_bound, SMALL_SAMPLE_THRESHOLD};
use crate::core::Finding;
use crate::parsers::Language;

// ---------- Registry ----------

/// Static metadata describing one external comparator. The registry
/// is a `&[BaselineSpec]` constant in this module so adding or
/// removing entries is a deliberate code change.
///
/// Spec: `docs/spec/sota-baselines-v0.md` §"Baseline registry" / F1.
#[derive(Debug, Clone, Copy)]
pub struct BaselineSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub detector_id: &'static str,
    pub supported_languages: &'static [Language],
    pub image_ref: &'static str,
    pub image_digest: &'static str,
    pub citation_key: &'static str,
}

/// v0 registry. Both adapters ship as scaffolding pending live
/// Docker pinning — when the live Docker images are built and
/// pushed, the actual `sha256:...` strings are committed under
/// `baselines/<name>/UPSTREAM.md` and copied here. The
/// `DigestMismatch` guard in [`run_baseline_docker`] is what makes
/// the placeholder safe to ship: any attempt to run an unmatched
/// image fails loudly rather than silently producing a comparison
/// against the wrong artefact.
pub const REGISTRY: &[BaselineSpec] = &[
    BaselineSpec {
        name: "sourcerercc",
        description: "SourcererCC scalable Type-3 clone detector (Sajnani et al. ICSE 2016)",
        detector_id: "clone-drift",
        supported_languages: &[Language::Rust, Language::Python],
        image_ref: "ghcr.io/ktrysmt/cntrdct-baselines/sourcerercc:v1.0",
        // Placeholder. Replaced with the real digest at release time per
        // baselines/sourcerercc/UPSTREAM.md. The DigestMismatch error variant
        // is what makes shipping a placeholder safe: a real run that does not
        // match this string aborts before producing comparison numbers.
        image_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        citation_key: "sajnani-icse-2016",
    },
    BaselineSpec {
        name: "pybuglab",
        description: "PyBugLab self-supervised Python bug detector (Allamanis et al. NeurIPS 2021)",
        detector_id: "arg-swap",
        supported_languages: &[Language::Python],
        image_ref: "ghcr.io/ktrysmt/cntrdct-baselines/pybuglab:v1.0",
        // Placeholder. Replaced with the real digest at release time per
        // baselines/pybuglab/UPSTREAM.md. Same DigestMismatch contract as
        // the SourcererCC entry: a real run that does not match aborts
        // before producing comparison numbers.
        image_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        citation_key: "allamanis-neurips-2021",
    },
];

/// Look up a baseline by `--baseline <name>`. Returns `None` for an
/// unknown name.
pub fn find_baseline(name: &str) -> Option<&'static BaselineSpec> {
    REGISTRY.iter().find(|b| b.name == name)
}

// ---------- NormalisedFinding ----------

/// One row of the JSONL stream the Docker image emits. The schema
/// is the smallest shape `cntrdct::eval::evaluate`'s matching rule
/// can consume; `raw` carries the upstream tool's original output
/// verbatim so a future schema change can re-derive normalised
/// rows from the committed artefact.
///
/// Spec: `docs/spec/sota-baselines-v0.md` F2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalisedFinding {
    pub tool: String,
    pub tool_version: String,
    pub file: PathBuf,
    pub line: u32,
    pub detector_id: String,
    pub raw: serde_json::Value,
}

// ---------- BaselineError ----------

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error at line {line}: {source}")]
    Parse {
        line: u32,
        #[source]
        source: serde_json::Error,
    },
    #[error("tool mismatch at line {line}: expected `{expected}`, got `{got}`")]
    ToolMismatch {
        expected: String,
        got: String,
        line: u32,
    },
    #[error("detector_id mismatch at line {line}: expected `{expected}`, got `{got}`")]
    DetectorIdMismatch {
        expected: String,
        got: String,
        line: u32,
    },
    #[error("image digest mismatch for {image}: expected `{expected}`, got `{got}`")]
    DigestMismatch {
        expected: String,
        got: String,
        image: String,
    },
    #[error("baseline `{tool}` exited with code {code}: {stderr}")]
    ExitCode {
        tool: String,
        code: i32,
        stderr: String,
    },
    #[error("docker not found on PATH; required to run baseline `{tool}`")]
    DockerNotFound { tool: String },
    #[error("unknown baseline name `{0}` — see `cntrdct eval --baseline help` for the registry")]
    UnknownBaseline(String),
    #[error("cached JSONL not found at {path}; run without --baselines-skip-run first")]
    CachedJsonlMissing { path: PathBuf },
}

// ---------- Loader (F3) ----------

/// Parse a JSONL file emitted by the baseline image (or by a cached
/// run under `benchmarks/baselines/<tag>/<name>.jsonl`).
///
/// Skips blank lines and `//`-prefixed comment lines. Returns
/// 1-based line numbers on parse failure, mirroring the
/// eval-v0 §F2 loader shape. Validates each row's `tool` and
/// `detector_id` against the registry entry; either mismatch is an
/// adapter bug and the run aborts.
///
/// Spec: `docs/spec/sota-baselines-v0.md` F3.
pub fn load_baseline_jsonl(
    path: &Path,
    expected_tool: &str,
    expected_detector_id: &str,
) -> Result<Vec<NormalisedFinding>, BaselineError> {
    let body = fs::read_to_string(path).map_err(|e| BaselineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut out = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let line_no = (i + 1) as u32;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let row: NormalisedFinding =
            serde_json::from_str(trimmed).map_err(|e| BaselineError::Parse {
                line: line_no,
                source: e,
            })?;
        if row.tool != expected_tool {
            return Err(BaselineError::ToolMismatch {
                expected: expected_tool.to_string(),
                got: row.tool,
                line: line_no,
            });
        }
        if row.detector_id != expected_detector_id {
            return Err(BaselineError::DetectorIdMismatch {
                expected: expected_detector_id.to_string(),
                got: row.detector_id,
                line: line_no,
            });
        }
        out.push(row);
    }
    Ok(out)
}

// ---------- Docker invocation (F4) ----------

/// Invoke the pinned Docker image for `spec` against `corpus_dir`,
/// writing the resulting JSONL to `out_jsonl_path`.
///
/// Verifies the image digest matches `spec.image_digest` before
/// spawning the container, and runs the container with
/// `--network=none --rm --read-only` plus a readonly bind mount of
/// the corpus and a writable bind mount of the scratch directory
/// (`out_jsonl_path.parent()`).
///
/// Spec: `docs/spec/sota-baselines-v0.md` F4 + CLI flag section.
pub fn run_baseline_docker(
    spec: &BaselineSpec,
    corpus_dir: &Path,
    out_jsonl_path: &Path,
) -> Result<(), BaselineError> {
    verify_image_digest(spec)?;

    let scratch_dir = out_jsonl_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&scratch_dir).map_err(|e| BaselineError::Io {
        path: scratch_dir.clone(),
        source: e,
    })?;

    let corpus_abs = fs::canonicalize(corpus_dir).map_err(|e| BaselineError::Io {
        path: corpus_dir.to_path_buf(),
        source: e,
    })?;
    let scratch_abs = fs::canonicalize(&scratch_dir).map_err(|e| BaselineError::Io {
        path: scratch_dir.clone(),
        source: e,
    })?;

    let mount_corpus = format!(
        "type=bind,src={},dst=/corpus,readonly",
        corpus_abs.display()
    );
    let mount_out = format!("type=bind,src={},dst=/out", scratch_abs.display());

    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--network=none",
        "--rm",
        "--read-only",
        "--mount",
        &mount_corpus,
        "--mount",
        &mount_out,
        spec.image_ref,
    ]);

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BaselineError::DockerNotFound {
                tool: spec.name.to_string(),
            }
        } else {
            BaselineError::Io {
                path: PathBuf::from("docker"),
                source: e,
            }
        }
    })?;

    if !output.status.success() {
        return Err(BaselineError::ExitCode {
            tool: spec.name.to_string(),
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(())
}

/// `docker inspect --format '{{index .RepoDigests 0}}'` and compare
/// against `spec.image_digest`. Surfaces `DockerNotFound` if the
/// CLI is missing; surfaces `DigestMismatch` if the image is
/// present but has a different digest than the spec pins.
fn verify_image_digest(spec: &BaselineSpec) -> Result<(), BaselineError> {
    let output = Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{index .RepoDigests 0}}",
            spec.image_ref,
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BaselineError::DockerNotFound {
                    tool: spec.name.to_string(),
                }
            } else {
                BaselineError::Io {
                    path: PathBuf::from("docker"),
                    source: e,
                }
            }
        })?;

    if !output.status.success() {
        return Err(BaselineError::ExitCode {
            tool: spec.name.to_string(),
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let got = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let want_substring = spec.image_digest;
    if !got.contains(want_substring) {
        return Err(BaselineError::DigestMismatch {
            expected: want_substring.to_string(),
            got,
            image: spec.image_ref.to_string(),
        });
    }
    Ok(())
}

// ---------- Comparison (F5) ----------

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct ToolMetrics {
    pub tool: String,
    pub tool_version: String,
    pub tp: u32,
    pub fp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub wilson_lower_precision: f64,
    pub wilson_lower_recall: f64,
    pub interval_method: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DetectorComparison {
    pub detector_id: String,
    pub corpus_name: String,
    pub cntrdct: ToolMetrics,
    pub baseline: ToolMetrics,
    pub concordance_tp_tp: u32,
    pub concordance_fp_fp: u32,
    pub expected_total: u32,
    pub corpus_size: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BaselineComparisonReport {
    pub release_tag: String,
    /// SHA-256 of `benchmarks/priors-default.json` as embedded in the
    /// binary. Surfaces alongside the comparison so a release-time
    /// consumer can verify the shipped priors against a known hash.
    pub priors_default_sha256: String,
    pub comparisons: Vec<DetectorComparison>,
}

/// Compute one [`DetectorComparison`] cell. `cntrdct_findings` are
/// the rows `cntrdct scan` produced; `baseline_findings` are the
/// normalised JSONL rows. `expected_lines` is the set of (file, line)
/// pairs from the manifest's `expected[]` entries whose
/// `detector_id` equals `spec.detector_id`; the matching rule
/// mirrors eval-v0 §F3 (exact line, exact file).
///
/// Spec: `docs/spec/sota-baselines-v0.md` F5.
#[allow(clippy::too_many_arguments)]
pub fn compare_one(
    spec: &BaselineSpec,
    corpus_name: &str,
    corpus_size: u32,
    expected_lines: &BTreeSet<(PathBuf, u32)>,
    cntrdct_findings: &[Finding],
    cntrdct_corpus_dir: &Path,
    cntrdct_version: &str,
    baseline_findings: &[NormalisedFinding],
    baseline_tool_version: &str,
) -> DetectorComparison {
    let cntrdct_keys: BTreeSet<(PathBuf, u32)> = cntrdct_findings
        .iter()
        .filter(|f| f.detector_id == spec.detector_id)
        .map(|f| {
            let rel = f
                .primary
                .file
                .strip_prefix(cntrdct_corpus_dir)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| f.primary.file.clone());
            (rel, f.primary.start_line)
        })
        .collect();

    let baseline_keys: BTreeSet<(PathBuf, u32)> = baseline_findings
        .iter()
        .filter(|r| r.detector_id == spec.detector_id)
        .map(|r| (r.file.clone(), r.line))
        .collect();

    let cntrdct = tool_metrics("cntrdct", cntrdct_version, &cntrdct_keys, expected_lines);
    let baseline = tool_metrics(
        spec.name,
        baseline_tool_version,
        &baseline_keys,
        expected_lines,
    );

    let concordance_tp_tp = cntrdct_keys
        .intersection(&baseline_keys)
        .filter(|k| expected_lines.contains(k))
        .count() as u32;
    let concordance_fp_fp = cntrdct_keys
        .intersection(&baseline_keys)
        .filter(|k| !expected_lines.contains(k))
        .count() as u32;

    DetectorComparison {
        detector_id: spec.detector_id.to_string(),
        corpus_name: corpus_name.to_string(),
        cntrdct,
        baseline,
        concordance_tp_tp,
        concordance_fp_fp,
        expected_total: expected_lines.len() as u32,
        corpus_size,
    }
}

fn tool_metrics(
    tool: &str,
    tool_version: &str,
    keys: &BTreeSet<(PathBuf, u32)>,
    expected: &BTreeSet<(PathBuf, u32)>,
) -> ToolMetrics {
    let tp = keys.intersection(expected).count() as u32;
    let fp = (keys.len() as u32).saturating_sub(tp);
    let fn_ = (expected.len() as u32).saturating_sub(tp);
    let precision = if tp + fp == 0 {
        0.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        0.0
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    // Spec text: "For cells with `tp + fp + fn < 30`, the Beta(1, 1)
    // Bayes-Laplace lower bound from Q-11 (`compute_lower_bound`) is
    // reported alongside." `compute_lower_bound(numerator, denom-numerator)`
    // — for precision pass (tp, fp); for recall pass (tp, fn).
    let (wilson_lower_precision, _) = compute_lower_bound(tp, fp);
    let (wilson_lower_recall, _) = compute_lower_bound(tp, fn_);
    let cell_n = tp + fp + fn_;
    let interval_method = if cell_n < SMALL_SAMPLE_THRESHOLD {
        "jeffreys"
    } else {
        "wilson"
    };
    ToolMetrics {
        tool: tool.to_string(),
        tool_version: tool_version.to_string(),
        tp,
        fp,
        fn_,
        precision,
        recall,
        f1,
        wilson_lower_precision,
        wilson_lower_recall,
        interval_method: interval_method.to_string(),
    }
}

// ---------- Report assembly ----------

/// Build a [`BaselineComparisonReport`] from one or more
/// [`DetectorComparison`] cells. Sorts `comparisons` by
/// `(detector_id, corpus_name)` so the serialised JSON is
/// byte-stable across runs over the same inputs (F6).
pub fn assemble_report(
    release_tag: &str,
    priors_default_json: &str,
    mut comparisons: Vec<DetectorComparison>,
) -> BaselineComparisonReport {
    comparisons.sort_by(|a, b| {
        a.detector_id
            .cmp(&b.detector_id)
            .then_with(|| a.corpus_name.cmp(&b.corpus_name))
    });
    BaselineComparisonReport {
        release_tag: release_tag.to_string(),
        priors_default_sha256: sha256_hex(priors_default_json),
        comparisons,
    }
}

/// SHA-256 of the input bytes, hex-encoded (lowercase). Used for the
/// H6.3 structural guard.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", byte).expect("hex write to String never fails");
    }
    s
}

// ---------- Cache path resolution ----------

/// Path the `--baselines-skip-run` flag reads from. Layout:
/// `<corpus_dir.parent>/baselines/<release_tag>/<baseline_name>.jsonl`.
///
/// The corpus directory's parent is the cntrdct repo root in the
/// shipped case (`benchmarks/audit-corpus` → repo root →
/// `benchmarks/baselines/<tag>/<name>.jsonl`), which matches the
/// spec's "v0 commits the per-tag JSONL under
/// `benchmarks/baselines/<tag>/`" line. The function falls back to
/// the corpus dir itself when no parent exists, which only happens
/// in test fixtures.
pub fn cached_jsonl_path(corpus_dir: &Path, release_tag: &str, baseline_name: &str) -> PathBuf {
    let base = corpus_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| corpus_dir.to_path_buf());
    base.join("baselines")
        .join(release_tag)
        .join(format!("{}.jsonl", baseline_name))
}

// ---------- Helpers consumed by lib.rs orchestration ----------

/// Extract the `(file, line)` set of expected entries for `detector_id`
/// from a manifest entry list. Accepts either the eval-v0 or the
/// recall-audit `expected[]` shape via a small trait-style closure
/// the caller supplies; this keeps `baselines` independent of both
/// `crate::eval` and `crate::recall_audit` schema choices.
pub fn expected_lines<E, F>(
    entries: &[E],
    detector_id: &str,
    project: F,
) -> BTreeSet<(PathBuf, u32)>
where
    F: Fn(&E) -> Vec<(PathBuf, String, u32)>,
{
    let mut out = BTreeSet::new();
    for entry in entries {
        for (file, det, line) in project(entry) {
            if det == detector_id {
                out.insert((file, line));
            }
        }
    }
    out
}

/// Generate the BTreeMap "tool_version -> count" payload used by the
/// CLI's stderr summary so a maintainer running `cntrdct eval
/// --baseline ...` sees at a glance whether each baseline produced
/// rows. Not part of the report JSON; debug-surface only.
pub fn baseline_row_counts(rows: &[NormalisedFinding]) -> BTreeMap<String, u32> {
    let mut out: BTreeMap<String, u32> = BTreeMap::new();
    for r in rows {
        *out.entry(r.tool.clone()).or_insert(0) += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_jsonl(path: &Path, lines: &[&str]) {
        fs::write(path, lines.join("\n") + "\n").expect("write");
    }

    fn synth_row(tool: &str, det: &str, file: &str, line: u32) -> String {
        json!({
            "tool": tool,
            "tool_version": "1.0-test",
            "file": file,
            "line": line,
            "detector_id": det,
            "raw": {}
        })
        .to_string()
    }

    #[test]
    fn load_baseline_jsonl_parses_valid_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.jsonl");
        write_jsonl(
            &p,
            &[
                &synth_row("sourcerercc", "clone-drift", "files/a.rs", 1),
                &synth_row("sourcerercc", "clone-drift", "files/b.rs", 5),
            ],
        );
        let rows = load_baseline_jsonl(&p, "sourcerercc", "clone-drift").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].file, PathBuf::from("files/a.rs"));
        assert_eq!(rows[1].line, 5);
    }

    #[test]
    fn load_baseline_jsonl_skips_blank_and_comment_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.jsonl");
        let body = format!(
            "// header\n\n{}\n\n// trailing\n",
            synth_row("sourcerercc", "clone-drift", "files/a.rs", 1)
        );
        fs::write(&p, body).unwrap();
        let rows = load_baseline_jsonl(&p, "sourcerercc", "clone-drift").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn load_baseline_jsonl_rejects_tool_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.jsonl");
        write_jsonl(&p, &[&synth_row("pybuglab", "arg-swap", "files/a.py", 1)]);
        let err = load_baseline_jsonl(&p, "sourcerercc", "clone-drift").unwrap_err();
        match err {
            BaselineError::ToolMismatch {
                expected,
                got,
                line,
            } => {
                assert_eq!(expected, "sourcerercc");
                assert_eq!(got, "pybuglab");
                assert_eq!(line, 1);
            }
            other => panic!("expected ToolMismatch, got {:?}", other),
        }
    }

    #[test]
    fn load_baseline_jsonl_rejects_detector_id_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.jsonl");
        write_jsonl(
            &p,
            &[&synth_row("sourcerercc", "arg-swap", "files/a.rs", 1)],
        );
        let err = load_baseline_jsonl(&p, "sourcerercc", "clone-drift").unwrap_err();
        assert!(matches!(err, BaselineError::DetectorIdMismatch { .. }));
    }

    #[test]
    fn load_baseline_jsonl_reports_one_based_line() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.jsonl");
        let body = format!(
            "{}\nthis is not json\n",
            synth_row("sourcerercc", "clone-drift", "files/a.rs", 1)
        );
        fs::write(&p, body).unwrap();
        let err = load_baseline_jsonl(&p, "sourcerercc", "clone-drift").unwrap_err();
        match err {
            BaselineError::Parse { line, .. } => assert_eq!(line, 2),
            other => panic!("expected Parse error, got {:?}", other),
        }
    }

    #[test]
    fn small_n_cell_reports_jeffreys() {
        // A cell with tp=1, fp=1, fn=1 -> n=3 < 30 -> jeffreys.
        let expected: BTreeSet<(PathBuf, u32)> =
            [(PathBuf::from("a.rs"), 10), (PathBuf::from("b.rs"), 20)]
                .into_iter()
                .collect();
        let keys: BTreeSet<(PathBuf, u32)> =
            [(PathBuf::from("a.rs"), 10), (PathBuf::from("c.rs"), 99)]
                .into_iter()
                .collect();
        let m = tool_metrics("cntrdct", "v", &keys, &expected);
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 1);
        assert_eq!(m.fn_, 1);
        assert_eq!(m.interval_method, "jeffreys");
    }

    #[test]
    fn large_n_cell_reports_wilson() {
        // Build 30 expected entries; tool catches them all -> tp = 30, fp = 0, fn = 0.
        let mut expected: BTreeSet<(PathBuf, u32)> = BTreeSet::new();
        for i in 0..30u32 {
            expected.insert((PathBuf::from(format!("f{i}.rs")), i + 1));
        }
        let keys = expected.clone();
        let m = tool_metrics("cntrdct", "v", &keys, &expected);
        assert_eq!(m.tp, 30);
        assert_eq!(m.interval_method, "wilson");
    }

    #[test]
    fn concordance_counts_split_tp_tp_and_fp_fp() {
        let spec = REGISTRY[0];
        let expected: BTreeSet<(PathBuf, u32)> =
            [(PathBuf::from("a.rs"), 10), (PathBuf::from("b.rs"), 20)]
                .into_iter()
                .collect();
        // cntrdct finds the TP at a:10 plus an FP at z:99.
        let cn = vec![
            mk_finding("clone-drift", "a.rs", 10),
            mk_finding("clone-drift", "z.rs", 99),
        ];
        // baseline finds the same TP plus the same FP.
        let bn = vec![
            NormalisedFinding {
                tool: "sourcerercc".into(),
                tool_version: "1.0".into(),
                file: PathBuf::from("a.rs"),
                line: 10,
                detector_id: "clone-drift".into(),
                raw: json!({}),
            },
            NormalisedFinding {
                tool: "sourcerercc".into(),
                tool_version: "1.0".into(),
                file: PathBuf::from("z.rs"),
                line: 99,
                detector_id: "clone-drift".into(),
                raw: json!({}),
            },
        ];
        let cmp = compare_one(
            &spec,
            "test-corpus",
            2,
            &expected,
            &cn,
            Path::new(""),
            "v0.4.0",
            &bn,
            "1.0",
        );
        assert_eq!(cmp.concordance_tp_tp, 1);
        assert_eq!(cmp.concordance_fp_fp, 1);
        assert_eq!(cmp.cntrdct.tp, 1);
        assert_eq!(cmp.cntrdct.fp, 1);
        assert_eq!(cmp.baseline.tp, 1);
        assert_eq!(cmp.baseline.fp, 1);
    }

    fn mk_finding(detector: &str, file: &str, line: u32) -> Finding {
        use crate::core::{AnomalyClass, Evidence, LanguageCitationStatus, Location, Severity};
        Finding {
            detector_id: detector.into(),
            primary: Location {
                file: PathBuf::from(file),
                start_line: line,
                end_line: line,
                start_col: 1,
                end_col: 1,
            },
            related: vec![],
            message: "synthetic".into(),
            raw_severity: Severity::Note,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec![],
                raw: json!({}),
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        }
    }

    #[test]
    fn sha256_hex_matches_known_value() {
        // Reference vector: SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn report_is_byte_stable_across_runs() {
        let spec = REGISTRY[0];
        let expected: BTreeSet<(PathBuf, u32)> =
            [(PathBuf::from("a.rs"), 10), (PathBuf::from("b.rs"), 20)]
                .into_iter()
                .collect();
        let cn = vec![mk_finding("clone-drift", "a.rs", 10)];
        let bn = vec![NormalisedFinding {
            tool: "sourcerercc".into(),
            tool_version: "1.0".into(),
            file: PathBuf::from("a.rs"),
            line: 10,
            detector_id: "clone-drift".into(),
            raw: json!({}),
        }];
        let cmp1 = compare_one(
            &spec,
            "x",
            2,
            &expected,
            &cn,
            Path::new(""),
            "v0.4.0",
            &bn,
            "1.0",
        );
        let cmp2 = cmp1.clone();
        let r1 = assemble_report("v0.4.0", "{}", vec![cmp1.clone(), cmp2.clone()]);
        let r2 = assemble_report("v0.4.0", "{}", vec![cmp2, cmp1]);
        let j1 = serde_json::to_string(&r1).unwrap();
        let j2 = serde_json::to_string(&r2).unwrap();
        assert_eq!(j1, j2);
    }

    #[test]
    fn cached_jsonl_path_layout_matches_spec() {
        let p = cached_jsonl_path(
            Path::new("benchmarks/audit-corpus"),
            "v0.4.0",
            "sourcerercc",
        );
        assert_eq!(
            p,
            PathBuf::from("benchmarks/baselines/v0.4.0/sourcerercc.jsonl")
        );
    }

    #[test]
    fn find_baseline_resolves_registry_name() {
        assert_eq!(
            find_baseline("sourcerercc").map(|b| b.name),
            Some("sourcerercc")
        );
        assert_eq!(
            find_baseline("pybuglab").map(|b| (b.name, b.detector_id)),
            Some(("pybuglab", "arg-swap"))
        );
        assert!(find_baseline("unknown").is_none());
    }

    #[test]
    fn pybuglab_registry_entry_matches_spec() {
        // Spec: docs/spec/sota-baselines-v0.md §"Baseline registry" plus F6.
        // PyBugLab is Python-only and maps to the `arg-swap` detector;
        // its citation key reuses the existing Layer 1 entry.
        let spec = find_baseline("pybuglab").expect("pybuglab in registry");
        assert_eq!(spec.detector_id, "arg-swap");
        assert_eq!(spec.supported_languages, &[Language::Python]);
        assert_eq!(spec.citation_key, "allamanis-neurips-2021");
        assert!(spec.image_ref.contains("pybuglab"));
        // Placeholder digest until live image is pinned per UPSTREAM.md.
        assert!(spec.image_digest.starts_with("sha256:"));
    }

    #[test]
    fn load_baseline_jsonl_parses_valid_pybuglab_rows() {
        // Test plan ID L2. Mirrors `load_baseline_jsonl_parses_valid_rows`
        // (L1) for the `pybuglab` -> `arg-swap` adapter.
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.jsonl");
        write_jsonl(
            &p,
            &[
                &synth_row("pybuglab", "arg-swap", "files/a.py", 7),
                &synth_row("pybuglab", "arg-swap", "files/b.py", 42),
            ],
        );
        let rows = load_baseline_jsonl(&p, "pybuglab", "arg-swap").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].file, PathBuf::from("files/a.py"));
        assert_eq!(rows[1].line, 42);
    }

    #[test]
    fn baseline_row_counts_aggregates_by_tool() {
        let rows = vec![
            NormalisedFinding {
                tool: "sourcerercc".into(),
                tool_version: "1.0".into(),
                file: PathBuf::from("a.rs"),
                line: 1,
                detector_id: "clone-drift".into(),
                raw: json!({}),
            },
            NormalisedFinding {
                tool: "sourcerercc".into(),
                tool_version: "1.0".into(),
                file: PathBuf::from("b.rs"),
                line: 2,
                detector_id: "clone-drift".into(),
                raw: json!({}),
            },
        ];
        let counts = baseline_row_counts(&rows);
        assert_eq!(counts.get("sourcerercc").copied(), Some(2));
    }
}
