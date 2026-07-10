//! Integration tests for I-1 gitignore-respecting file discovery in
//! `cntrdct scan` and its `--no-ignore` opt-out.
//!
//! Spec: `docs/spec/scan-ignore-v0.md`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::tempdir;

/// Triggers `unreachable-after-terminator` deterministically (same
/// shape as `tests/scan_baseline.rs`).
const DEAD_CODE: &str = "fn f() { return; bar(); }\n";

fn cntrdct_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

fn scan(args: &[&str]) -> Output {
    Command::new(cntrdct_bin())
        .arg("scan")
        .args(args)
        .output()
        .expect("invoke cntrdct scan")
}

/// The `finding.primary.file` paths of every reported finding.
fn finding_files(out: &Output) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let findings: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "stdout must be a JSON findings array: {}\nstdout: {}",
            e, stdout
        )
    });
    findings
        .iter()
        .map(|f| {
            f["finding"]["primary"]["file"]
                .as_str()
                .expect("finding.primary.file is a string")
                .to_string()
        })
        .collect()
}

#[test]
fn gitignored_files_are_skipped_by_default_and_scanned_with_no_ignore() {
    let dir = tempdir().unwrap();
    // Git-derived ignore rules only apply inside a git repository
    // (`require_git`, ripgrep semantics); an empty `.git` dir marks
    // the root as one.
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".gitignore"), "vendored.rs\n").unwrap();
    fs::write(dir.path().join("tracked.rs"), DEAD_CODE).unwrap();
    fs::write(dir.path().join("vendored.rs"), DEAD_CODE).unwrap();
    let root = dir.path().to_str().unwrap();

    let out = scan(&[root]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files = finding_files(&out);
    assert!(
        !files.is_empty(),
        "the non-ignored file must still produce its finding"
    );
    assert!(
        files.iter().all(|f| f.ends_with("tracked.rs")),
        "a gitignored file must not be scanned by default: {:?}",
        files
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("in 1 file(s)"), "got: {}", stderr);

    let out = scan(&[root, "--no-ignore"]);
    assert!(out.status.success());
    let files = finding_files(&out);
    assert!(
        files.iter().any(|f| f.ends_with("vendored.rs")),
        "--no-ignore must restore gitignored files: {:?}",
        files
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("in 2 file(s)"), "got: {}", stderr);
}

#[test]
fn git_internals_are_never_scanned() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".git").join("planted.rs"), DEAD_CODE).unwrap();
    fs::write(dir.path().join("app.rs"), DEAD_CODE).unwrap();
    let root = dir.path().to_str().unwrap();

    for args in [vec![root], vec![root, "--no-ignore"]] {
        let out = scan(&args);
        assert!(out.status.success());
        let files = finding_files(&out);
        assert!(
            files.iter().all(|f| !f.ends_with("planted.rs")),
            ".git/ contents must never be scanned (args {:?}): {:?}",
            args,
            files
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("in 1 file(s)"),
            "args {:?}, got: {}",
            args,
            stderr
        );
    }
}

#[test]
fn gitignore_outside_a_git_repo_is_not_applied() {
    // ripgrep semantics (`require_git` default): outside a git
    // repository a `.gitignore` file has no effect. Pins the default
    // so tempdir-based fixtures elsewhere in the suite keep scanning
    // every file.
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".gitignore"), "vendored.rs\n").unwrap();
    fs::write(dir.path().join("vendored.rs"), DEAD_CODE).unwrap();
    let root = dir.path().to_str().unwrap();

    let out = scan(&[root]);
    assert!(out.status.success());
    let files = finding_files(&out);
    assert!(
        files.iter().any(|f| f.ends_with("vendored.rs")),
        "without a .git dir the .gitignore must be inert: {:?}",
        files
    );
}

#[test]
fn dot_ignore_applies_even_outside_a_git_repo() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".ignore"), "vendored.rs\n").unwrap();
    fs::write(dir.path().join("vendored.rs"), DEAD_CODE).unwrap();
    fs::write(dir.path().join("kept.rs"), DEAD_CODE).unwrap();
    let root = dir.path().to_str().unwrap();

    let out = scan(&[root]);
    assert!(out.status.success());
    let files = finding_files(&out);
    assert!(
        !files.is_empty() && files.iter().all(|f| f.ends_with("kept.rs")),
        ".ignore must apply with or without git: {:?}",
        files
    );
}

#[test]
fn eval_scans_gitignored_corpus_files() {
    // I-1: `eval` (and `calibrate --audit-recall`) walk their corpus
    // directory with no-ignore semantics — a corpus is an explicit
    // input, and scratch corpora are commonly gitignored, so ignore
    // rules must not silently score every manifest entry as a miss.
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".gitignore"), "*\n").unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let manifest = dir.path().join("manifest.jsonl");
    fs::write(
        &manifest,
        r#"{"file": "dead.rs", "expected": [{"detector_id": "unreachable-after-terminator", "line": 1}]}"#,
    )
    .unwrap();

    let report = cntrdct::run_eval(dir.path(), &manifest).expect("run_eval");
    assert_eq!(
        report.overall.recall, 1.0,
        "an ignore-everything corpus must still be walked by eval: {:?}",
        report
    );
}

#[test]
fn explicit_file_argument_bypasses_ignore_rules() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".gitignore"), "vendored.rs\n").unwrap();
    let file = dir.path().join("vendored.rs");
    fs::write(&file, DEAD_CODE).unwrap();

    let out = scan(&[file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(
        !finding_files(&out).is_empty(),
        "an explicitly named file is always scanned, ignored or not"
    );
}
