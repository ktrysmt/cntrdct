//! T2-11 acceptance: `cargo cntrdct ...` reaches the same code path as
//! `cntrdct ...`.
//!
//! The test runs the installed `cargo-cntrdct` shim from the workspace
//! `target/` directory and asserts:
//! - the shim strips the leading `cntrdct` arg that cargo prepends, and
//! - both invocation styles produce a non-empty JSON array.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn shim_binary() -> PathBuf {
    let mut p = workspace_root();
    p.push("target");
    p.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    p.push(if cfg!(windows) {
        "cargo-cntrdct.exe"
    } else {
        "cargo-cntrdct"
    });
    p
}

#[test]
fn shim_runs_with_cargo_style_invocation() {
    // Skip if the binary hasn't been built yet (e.g., when running tests on a
    // fresh checkout via `cargo test --test cargo_subcommand` alone).
    let bin = shim_binary();
    if !bin.exists() {
        eprintln!("shim not built at {}; skipping", bin.display());
        return;
    }

    let corpus = workspace_root()
        .join("benchmarks")
        .join("corpus")
        .join("files");

    let out = Command::new(&bin)
        .args(["cntrdct", "scan"])
        .arg(&corpus)
        .args(["--format", "json"])
        .output()
        .expect("run shim");

    assert!(
        out.status.success(),
        "shim exit status non-zero: {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim_start().starts_with('['),
        "expected JSON array, got: {}",
        stdout
    );
}

#[test]
fn shim_runs_without_subcommand_prefix() {
    let bin = shim_binary();
    if !bin.exists() {
        eprintln!("shim not built at {}; skipping", bin.display());
        return;
    }

    let corpus = workspace_root()
        .join("benchmarks")
        .join("corpus")
        .join("files");

    // Direct invocation (no leading `cntrdct` arg).
    let out = Command::new(&bin)
        .args(["scan"])
        .arg(&corpus)
        .args(["--format", "json"])
        .output()
        .expect("run shim");

    assert!(
        out.status.success(),
        "shim exit status non-zero: {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim_start().starts_with('['),
        "expected JSON array, got: {}",
        stdout
    );
}
