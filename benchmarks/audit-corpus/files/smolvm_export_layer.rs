// Source: https://github.com/smol-machines/smolvm/blob/019654bd61d9051fc42df7bbcdee446cc3e31ae1/crates/smolvm-agent/src/storage.rs
// License: Apache-2.0
// Note: minimal extract of one top-level `pub fn` from smol-machines/smolvm@019654bd61d9051fc42df7bbcdee446cc3e31ae1 crates/smolvm-agent/src/storage.rs (upstream lines 1413-1452, audit-corpus lines 5-44 after the 3-line header + 1 blank-line offset). The function carries a four-line `///` doc block (upstream lines 1413-1417 / corpus lines 5-9) whose later half (after the blank `///` separator at upstream line 1414 / corpus line 6) reads `DEPRECATED: Prefer streaming export via `find_layer_path()` + piped tar. This function creates a temp tar file that can fill the storage disk for large layers. Kept for backward compatibility.` but does NOT carry the `#[deprecated]` runtime attribute the Rust deprecation lints honour, so downstream consumers receive no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment": deprecation prose without `#[deprecated]` attribute) bug shape, the same one cntrdct's batch-3 `sidan-lab/whisky-archive` con_str* family, batch-11 `rusticata/tls-parser` `parse_tls_handshake_*next_protocol` family, batch-12 `glium/glium` `validate` flag, batch-13 `rust-lang/pkg-config-rs` `find_library` flag, batch-20 `MystenLabs/sui` `mysten-metrics/metered_channel` `channel`/`channel_with_total` flag, batch-21 `mcgoo/vcpkg-rs` `probe_package` flag, batch-22 `overdrivenpotato/rust-vst2` `process_deprecated` flag, and batch-23 `always-further/nono` `warn_for_deprecated_flags` flag. Diversifies `comment-code`'s Pattern C audit evidence from eight upstreams to nine upstreams (whisky-archive 4 + tls-parser 2 + glium 1 + pkg-config-rs 1 + sui mysten-metrics 2 + vcpkg-rs 1 + rust-vst2 1 + nono 1 + smolvm 1) on nine unrelated codebases (Cardano Plutus-data helpers + TLS NextProtocol parsers + OpenGL draw-parameter check + Unix pkg-config bindings + async-channel metrics + Windows vcpkg bindings + VST 2.4 audio plugin host + capability-based sandbox CLI + portable lightweight VM image layer storage). The function body is a typical in-tree implementation that calls `find_layer_path(image_digest, layer_index)?` to locate the layer source directory, builds a temporary tar path under `STORAGE_ROOT/tmp/`, spawns `tar -cf <tar_path> -C <layer_dir> .` via `Command::new("tar")`, and returns the resulting `PathBuf` — the function still works (and is "kept for backward compatibility" per the doc) but is being phased out in favour of streaming export through `find_layer_path()` + an externally-piped tar process to avoid filling the storage disk with intermediate tar files for large layers. This is the same body-shape category as batch-11 tls-parser, batch-12 glium, and batch-20 sui mysten-metrics (in-tree implementation rather than delegation to the replacement), on a domain unrelated to those three. cntrdct's spec F5 Pattern C trigger fires on the case-folded `deprecated` substring; the preceding-siblings walker finds zero attribute items adjacent to the function so the `#[deprecated]` lint is not honoured. The function signature `pub fn export_layer(image_digest: &str, layer_index: usize) -> Result<PathBuf>` returns `Result<PathBuf>` (carrying the `Result` substring required by spec F3's return-type negation), so even if the doc contained a Pattern A trigger phrase the return-type negation would suppress Pattern A; the doc contains no Pattern A trigger phrase (none of `returns err` / `returns result` / `may fail` / `fallible` / `returns option` / `may return none`) so Pattern A does not fire on either grounds; the doc contains no `panic` substring so spec F4 Pattern B does not fire — only Pattern C fires. Unrelated imports (`std::path::PathBuf`, `tracing::info`, `std::process::Command`, `std::path::Path`), the surrounding `find_layer_path` / `get_layer_digest` / `purge_all_images` / `garbage_collect` functions, the file-scope constants `STORAGE_ROOT` / `LAYERS_DIR` / `MANIFESTS_DIR`, and the `StorageError::new` constructor are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern C check operate per-fn; tree-sitter parses unresolved identifiers (`Result`, `PathBuf`, `find_layer_path`, `Path`, `STORAGE_ROOT`, `Command`, `info`, `StorageError`) syntactically without requiring resolution. smolvm is the portable, lightweight, self-contained VM tooling domain (the agent crate's storage module manages OCI-style image layers on disk for the VM runtime), unrelated to the prior eight Pattern C domains (Cardano Plutus-data, TLS NextProtocol, OpenGL draw-parameters, Unix pkg-config, async-channel metrics, Windows vcpkg, VST 2.4 audio plugin host, capability-based sandbox CLI). SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Export a layer as a tar file on the storage disk.
///
/// DEPRECATED: Prefer streaming export via `find_layer_path()` + piped tar.
/// This function creates a temp tar file that can fill the storage disk for
/// large layers. Kept for backward compatibility.
pub fn export_layer(image_digest: &str, layer_index: usize) -> Result<PathBuf> {
    let layer_dir = find_layer_path(image_digest, layer_index)?;
    let layer_id = layer_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let root = Path::new(STORAGE_ROOT);
    let tmp_dir = root.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    let tar_path = tmp_dir.join(format!("layer-{}.tar", &layer_id[..12.min(layer_id.len())]));

    info!(
        layer_id = %layer_id,
        output = %tar_path.display(),
        "exporting layer as tar (temp file)"
    );

    let status = Command::new("tar")
        .args(["-cf"])
        .arg(&tar_path)
        .arg("-C")
        .arg(&layer_dir)
        .arg(".")
        .status()?;

    if !status.success() {
        return Err(StorageError::new(format!(
            "failed to create tar archive for layer {}",
            layer_id
        )));
    }

    Ok(tar_path)
}
