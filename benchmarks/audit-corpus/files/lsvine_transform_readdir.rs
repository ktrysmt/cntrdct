// Source: https://github.com/autofitcloud/lsvine/blob/2b524aa431478554ea4cff8732a8e914c61c8d2a/src/vecpath2vecl1dir_iterators.rs
// License: Apache-2.0
// Note: minimal extract of one top-level `pub fn` from autofitcloud/lsvine@2b524aa431478554ea4cff8732a8e914c61c8d2a src/vecpath2vecl1dir_iterators.rs (upstream lines 56-102, audit-corpus lines 5-51 after the 3-line header + 1 blank-line offset). The function carries a thirteen-line `///` doc block (upstream lines 56-68 / corpus lines 5-17) whose first line reads `DEPRECATED in favor of RDAdapter1` but does NOT carry the `#[deprecated]` runtime attribute the Rust deprecation lints honour, so downstream consumers receive no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment": deprecation prose without `#[deprecated]` attribute) bug shape, the same one cntrdct's batch-3 `sidan-lab/whisky-archive` con_str* family, batch-11 `rusticata/tls-parser` `parse_tls_handshake_*next_protocol` family, batch-12 `glium/glium` `validate` flag, batch-13 `rust-lang/pkg-config-rs` `find_library` flag, batch-20 `MystenLabs/sui` `mysten-metrics/metered_channel` `channel`/`channel_with_total` flag, batch-21 `mcgoo/vcpkg-rs` `probe_package` flag, batch-22 `overdrivenpotato/rust-vst2` `process_deprecated` flag, batch-23 `always-further/nono` `warn_for_deprecated_flags` flag, batch-24 `smol-machines/smolvm` `export_layer` flag, and batch-25 `anyme123/Any-code` `decode_project_path` flag. Diversifies `comment-code`'s Pattern C audit evidence from ten upstreams to eleven upstreams (whisky-archive 4 + tls-parser 2 + glium 1 + pkg-config-rs 1 + sui mysten-metrics 2 + vcpkg-rs 1 + rust-vst2 1 + nono 1 + smolvm 1 + Any-code 1 + lsvine 1) on eleven unrelated codebases (Cardano Plutus-data helpers + TLS NextProtocol parsers + OpenGL draw-parameter check + Unix pkg-config bindings + async-channel metrics wrapper + Windows vcpkg bindings + VST 2.4 audio plugin host + capability-based sandbox CLI + portable lightweight VM image layer storage + Tauri-based Claude/Codex CLI session viewer + `tree -L 2`-style directory tree CLI iterator adapter). The function body falls into the existing in-tree-body sub-shape within Pattern C: it builds an iterator chain by `.filter`/`.map`-ing over the `std::fs::ReadDir` source (quietly skipping `Result::Err` entries, mapping each successful `DirEntry` to its `PathBuf`, wrapping each into a `PathBufWrap` struct via `PathBufWrap::new`, filtering out paths whose filename starts with `.`, and dropping entries whose target neither `is_file()` nor `is_dir()` while printing a `Path doesnt exist:` warning via `println!`) — re-implementing in free-function-with-closure-chain style the same per-entry filtering that the replacement struct `RDAdapter1` provides via an `impl Iterator for RDAdapter1` `fn next()` state-machine, both retained in the same file `src/vecpath2vecl1dir_iterators.rs` while the upstream migrates internal callers (see the companion `RDAdapter2` and the commented-out `RDAdapter3` further down the source file) to the struct-based form before deletion. This is the same body-shape category as batch-11 tls-parser, batch-12 glium, batch-20 sui mysten-metrics, batch-24 smolvm, and batch-25 Any-code (in-tree implementation rather than delegation to the replacement), on a sixth unrelated upstream — broadening in-tree-body audit coverage from five upstreams to six without introducing a new Pattern C body-shape sub-shape; the body-shape footprint within Pattern C stays at the four shapes saturated by batches 22 and 23 (delegate-body, in-tree-body, stub-body, meta-deprecation-warning-emitter). Within the in-tree-body sub-shape itself, lsvine introduces a new structural variant — replacement-targets-a-struct-not-a-function: prior in-tree-body upstreams (tls-parser, glium, sui mysten-metrics, smolvm, Any-code) all name a replacement free function (`parse_*`, `find_*`, `monitored_mpsc::channel`, `find_layer_path` + piped tar, `get_project_path_from_sessions`), whereas lsvine names a replacement iterator-adapter struct (`RDAdapter1`) intended to be instantiated via `RDAdapter1::new(...)` and consumed through its `Iterator` impl. cntrdct's spec F5 Pattern C check does not interpret what the replacement is — it only inspects the doc and adjacent attributes — so the function-replaced-by-struct case fires identically to the function-replaced-by-function cases, confirming again the syntactic-only design. cntrdct's spec F5 Pattern C trigger fires on the case-folded `deprecated` substring; the preceding-siblings walker finds zero attribute items adjacent to the function (no `#[deprecated]`, no `#[doc(hidden)]`, no `#[track_caller]` — the function carries no top-level attribute at all) so the `#[deprecated]` lint is not honoured. The function signature `pub fn transform_readdir(fs_readdir: std::fs::ReadDir) -> impl Iterator<Item = PathBufWrap>` returns `impl Iterator<Item = PathBufWrap>` (a non-`Result`/`Option` `impl Trait` return, the literal substrings `Result`/`Option` do not appear in the return-type text), so spec F3 Pattern A's return-type negation passes; the doc contains no Pattern A trigger phrase (none of `returns err` / `returns result` / `may fail` / `fallible` / `returns option` / `may return none`) so Pattern A does not fire either way; the doc contains no `panic` substring so spec F4 Pattern B does not fire — only Pattern C fires. Note that the function body contains the substring `unwrap` (in `.map(|e| e.unwrap().path())` on the upstream-line-80 closure) which would qualify as a Pattern B body marker if Pattern B's doc-trigger fired — but it does not, because the doc has no `panic` substring, so the body-marker negation is moot here; this is the dual of batch-17 wasmtime `roundtrip` where the doc does have `panic` but the body's `unwrap`/`assert_eq!` markers suppress Pattern B; lsvine `transform_readdir` is the inverse — body has `unwrap` but doc has no `panic` trigger, so the body marker is inert from cntrdct's perspective. Unrelated imports (`std::cmp`, `std::path::PathBuf`, the `pub use crate::level1dir;` and `pub use crate::longest_common_prefix;` re-exports that the rest of the source file depends on), the surrounding `PathBufWrap` struct + its `impl` + the companion `RDAdapter1` / `RDAdapter2` / commented-out `RDAdapter3` iterator-adapter structs and their `impl Iterator` bodies are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern C check operate per-fn; tree-sitter parses unresolved identifiers (`PathBufWrap`, `Iterator`, `std::fs::ReadDir`) syntactically without requiring resolution. autofitcloud/lsvine is the `tree -L 2` with less empty screen space CLI domain (a Rust rewrite of the directory tree-listing utility that contracts long common filename prefixes to keep the output narrow), unrelated to the prior ten Pattern C domains (Cardano Plutus-data, TLS NextProtocol, OpenGL draw-parameters, Unix pkg-config, async-channel metrics, Windows vcpkg, VST 2.4 audio plugin host, capability-based sandbox CLI, portable lightweight VM image layer storage, Tauri-based AI-coding-tool viewer). SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// DEPRECATED in favor of RDAdapter1
/// An iterator adapter that takes an iterator std::fs::ReadDir and:
/// - Consumes it into a collection of DirEntry https://doc.rust-lang.org/std/fs/struct.DirEntry.html
/// - Maps them to PathBuf
/// - skips paths whose filename starts with '.'
/// - skips paths that don't exist on disk
/// - converts it back to an iterator of PathBuf
///   (to stay in the iterator world later and use inheritance on the class's
///    iteration function rather than deal with collections)
///
/// Links
/// https://doc.rust-lang.org/std/fs/struct.ReadDir.html
/// https://doc.rust-lang.org/std/iter/index.html#adapters
pub fn transform_readdir(fs_readdir: std::fs::ReadDir) -> impl Iterator<Item = PathBufWrap> {
    // list contents of path
    // method 1: http://stackoverflow.com/questions/26076005/ddg#26084812
    // let level1_paths = fs::read_dir(args.path).unwrap();
    // method 2: https://doc.rust-lang.org/std/fs/fn.read_dir.html
    // TODO use partition instead of collect
    // https://www.reddit.com/r/rust/comments/eleleu/my_first_cli_in_rust_lsvine_list_contents_of/fditvjp
    // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.partition
    // Check the docs even/odd example
    let level1_paths = fs_readdir
                             .filter(|res| res.as_ref().ok().is_some()) // quietly skip erroneous entries
                             .map(|e| e.unwrap().path())
                             ;

    // map to PathBufWrap containing filenames
    level1_paths
           // quietly skip None values, like pandas skipna
           .filter(|p| p.file_name().is_some())
           // quietly skip errors
           .filter(|p| p.file_name().and_then(|x| x.to_str() ).is_some())
           // map to filenames (not Option<...>)
           // file_name returns Option: https://doc.rust-lang.org/std/option/index.html
           .map(PathBufWrap::new)
           // skip paths filenames that start with .
           .filter(|pbw| !pbw.file_name.starts_with('.'))
           // skip paths that don't exist on-disk
           .filter(|pbw| {
               if !pbw.path_buf.is_file() && !pbw.path_buf.is_dir() {
                 println!("Path doesnt exist: {}. Skipping", pbw.file_name);
                 return false;
               }
               true
           })
}
