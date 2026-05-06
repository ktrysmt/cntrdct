//! `cargo-cntrdct` shim.
//!
//! Cargo recognises any executable named `cargo-<name>` on `PATH` as the
//! implementation of `cargo <name>`. When invoked that way Cargo passes the
//! subcommand name as the first positional arg, e.g. `cargo cntrdct scan .`
//! reaches us as `["cargo-cntrdct", "cntrdct", "scan", "."]`.
//!
//! This shim drops the leading `cntrdct` arg (when present), then re-execs
//! the main `cntrdct` binary with the remaining arguments. Re-exec is used
//! instead of calling `cntrdct::main_entry` directly so the user-visible
//! exit codes, signal handling, and stdio behaviour are exactly identical to
//! `cntrdct ...`.

use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();

    // When invoked via `cargo cntrdct ...`, Cargo prepends our subcommand
    // name. Strip it so users can still call `cargo-cntrdct ...` directly
    // without the redundant arg leaking through.
    if args.first().map(String::as_str) == Some("cntrdct") {
        args.remove(0);
    }

    // Locate the sibling `cntrdct` binary by reusing the directory of the
    // current executable. Falls back to a bare `cntrdct` lookup on PATH if
    // current_exe() fails (vanishingly rare on supported platforms).
    let exe = env::current_exe().ok();
    let cntrdct_path = exe.as_ref().and_then(|p| p.parent()).map(|dir| {
        let mut p = dir.to_path_buf();
        p.push(if cfg!(windows) {
            "cntrdct.exe"
        } else {
            "cntrdct"
        });
        p
    });

    let status = match cntrdct_path {
        Some(p) if p.exists() => Command::new(p).args(&args).status(),
        _ => Command::new("cntrdct").args(&args).status(),
    };

    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("cargo-cntrdct: failed to invoke cntrdct: {}", e);
            ExitCode::from(127)
        }
    }
}
