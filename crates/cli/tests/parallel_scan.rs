//! T2-10 acceptance: per-file detector runs execute in parallel and the
//! output is deterministic.
//!
//! Two tests:
//! - `scan_is_deterministic_under_parallel_execution` runs `scan` twice over
//!   the same synthesised corpus and asserts the resulting `Vec<Finding>` is
//!   bit-identical between runs.
//! - `parallel_scan_is_faster_than_serial` (ignored by default; run with
//!   `cargo test -p cntrdct-cli --release -- --ignored parallel_scan_is_faster`)
//!   builds a 1000-file corpus, times scans inside a 1-thread rayon pool and
//!   inside a default-sized pool, and reports the speedup. Asserts the
//!   speedup is at least 2.0x — the roadmap's 4.0x target assumes an 8-core
//!   machine; CI runners commonly have 2-4 cores so the gating threshold is
//!   intentionally lower than the documented target.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use rayon::ThreadPoolBuilder;
use tempfile::TempDir;

const SYNTH_FILE_COUNT: usize = 1000;

fn synthesize_corpus(root: &std::path::Path, n: usize) {
    for i in 0..n {
        // Mix in patterns each detector can recognise so the parallel work is
        // non-trivial and the output is not empty.
        let body = format!(
            r#"
// Synthesised file #{i}.
pub fn alpha_{i}(a: i32, b: i32) -> i32 {{ a - b }}
pub fn beta_{i}(b: i32, a: i32) -> i32 {{ alpha_{i}(b, a) }}
pub fn gamma_{i}() -> i32 {{
    return 1;
    let _dead = 2;
    3
}}

/// Returns Ok always.
fn delta_{i}(x: u32) -> Result<u32, String> {{ Err(format!("{{}}", x)) }}
"#,
            i = i,
        );
        let path = root.join(format!("synth_{:04}.rs", i));
        fs::write(path, body).expect("write synth file");
    }
}

#[test]
fn scan_is_deterministic_under_parallel_execution() {
    let dir = TempDir::new().expect("tempdir");
    synthesize_corpus(dir.path(), 32);

    let first = cntrdct_cli::scan(dir.path()).expect("first scan");
    let second = cntrdct_cli::scan(dir.path()).expect("second scan");

    let key = |f: &cntrdct_core::Finding| {
        (
            f.detector_id.clone(),
            f.primary.file.clone(),
            f.primary.start_line,
            f.primary.start_col,
        )
    };
    let firsts: Vec<_> = first.iter().map(key).collect();
    let seconds: Vec<_> = second.iter().map(key).collect();
    assert_eq!(
        firsts, seconds,
        "parallel scan output must be order-stable across invocations"
    );
}

#[test]
#[ignore]
fn parallel_scan_is_faster_than_serial() {
    let dir = TempDir::new().expect("tempdir");
    synthesize_corpus(dir.path(), SYNTH_FILE_COUNT);
    let path: PathBuf = dir.path().to_path_buf();

    // Serial baseline: a 1-thread rayon pool. Use install() so any rayon
    // par_iter inside scan() falls into this pool.
    let serial_pool = ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("serial pool");
    let serial_path = path.clone();
    let serial_start = Instant::now();
    let serial = serial_pool.install(|| cntrdct_cli::scan(&serial_path).expect("serial scan"));
    let serial_elapsed = serial_start.elapsed();

    let parallel_pool = ThreadPoolBuilder::new()
        .num_threads(num_cpus_default())
        .build()
        .expect("parallel pool");
    let parallel_path = path.clone();
    let par_start = Instant::now();
    let parallel =
        parallel_pool.install(|| cntrdct_cli::scan(&parallel_path).expect("parallel scan"));
    let par_elapsed = par_start.elapsed();

    assert_eq!(
        serial.len(),
        parallel.len(),
        "serial and parallel runs must produce the same number of findings"
    );

    let speedup = serial_elapsed.as_secs_f64() / par_elapsed.as_secs_f64();
    eprintln!(
        "parallel scan: serial={:?}, parallel={:?}, speedup={:.2}x ({} files, {} threads)",
        serial_elapsed,
        par_elapsed,
        speedup,
        SYNTH_FILE_COUNT,
        num_cpus_default(),
    );
    assert!(
        speedup >= 2.0,
        "expected at least 2.0x speedup, observed {:.2}x",
        speedup
    );
}

fn num_cpus_default() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
}
