// Source: https://github.com/zarrs/zarrs/blob/3b944c57a0b7af127ae73ea250d3ffce60e51f0b/zarrs_data_type/src/codec_traits/bitround.rs
// License: MIT OR Apache-2.0
// Note: minimal extract of six top-level `pub fn` items (`round_bytes_int16`, `round_bytes_int32`, `round_bytes_int64`, `round_bytes_float16`, `round_bytes_float32`, `round_bytes_float64`) plus three referenced helper `const fn` items (`round_bits16`, `round_bits32`, `round_bits64`) from zarrs/zarrs@3b944c57a0b7af127ae73ea250d3ffce60e51f0b zarrs_data_type/src/codec_traits/bitround.rs (helpers at upstream lines 14-45 included so the Pattern B function bodies parse with resolved call targets; the six labelled bug instances sit at upstream lines 58 / 70 / 82 / 94 / 106 / 118 and at corpus lines 42 / 54 / 66 / 78 / 90 / 102 after the 3-line provenance header + 1 blank-line offset; the helpers carry no doc comment so cntrdct's comment-code per-fn loop skips them per spec F2). Each `round_bytes_int*` and `round_bytes_float*` function carries a `///` doc block whose `# Panics` section reads `Panics if \`bytes.len()\` is not a multiple of N.` for the corresponding N in {2, 4, 8}, but the body uses `bytes.as_chunks_mut::<N>().0` which never panics on non-multiple-of-N lengths — `slice::as_chunks_mut::<N>()` (stabilised in Rust 1.88.0, 2025-06) returns `(&mut [[T; N]], &mut [T])` where the second tuple element is the remainder with length strictly less than N; the upstream code accesses only `.0` (the chunked arrays) and silently discards the remainder, so trailing bytes that don't form a complete N-byte chunk are ignored rather than triggering the documented panic. The textbook Tan SOSP 2007 §3.2 Pattern B ("bad comment": panic claim without panicking constructs in the body) bug shape on a fifth permissive-licensed Rust upstream — Zarr-format data-type bindings, dual-licensed MIT OR Apache-2.0, the first audit-corpus entries on Pattern B after eight Pattern C entries on four prior upstreams (whisky-archive con_str* family, tls-parser parse_tls_handshake_*next_protocol family, glium validate, pkg-config-rs find_library). Unrelated trait definitions, `define_data_type_support!` macro, `_impl_bitround_codec!` macro_rules export, and the surrounding bitround codec infrastructure are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern B body-text substring check operate per-fn; tree-sitter parses the const generic turbofish syntax (`<N>`) cleanly without requiring resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

const fn round_bits16(mut input: u16, keepbits: u32, maxbits: u32) -> u16 {
    if keepbits < maxbits {
        let maskbits = maxbits - keepbits;
        let all_set = u16::MAX;
        let mask = (all_set >> maskbits) << maskbits;
        let half_quantum1 = (1 << (maskbits - 1)) - 1;
        input = input.saturating_add(((input >> maskbits) & 1) + half_quantum1) & mask;
    }
    input
}

const fn round_bits32(mut input: u32, keepbits: u32, maxbits: u32) -> u32 {
    if keepbits < maxbits {
        let maskbits = maxbits - keepbits;
        let all_set = u32::MAX;
        let mask = (all_set >> maskbits) << maskbits;
        let half_quantum1 = (1 << (maskbits - 1)) - 1;
        input = input.saturating_add(((input >> maskbits) & 1) + half_quantum1) & mask;
    }
    input
}

const fn round_bits64(mut input: u64, keepbits: u32, maxbits: u32) -> u64 {
    if keepbits < maxbits {
        let maskbits = maxbits - keepbits;
        let all_set = u64::MAX;
        let mask = (all_set >> maskbits) << maskbits;
        let half_quantum1 = (1 << (maskbits - 1)) - 1;
        input = input.saturating_add(((input >> maskbits) & 1) + half_quantum1) & mask;
    }
    input
}

/// Helper to round 16-bit integer values (from MSB).
///
/// # Panics
/// Panics if `bytes.len()` is not a multiple of 2.
pub fn round_bytes_int16(bytes: &mut [u8], keepbits: u32) {
    for chunk in bytes.as_chunks_mut::<2>().0 {
        let element = u16::from_ne_bytes(*chunk);
        let rounded = round_bits16(element, keepbits, 16 - element.leading_zeros());
        chunk.copy_from_slice(&u16::to_ne_bytes(rounded));
    }
}

/// Helper to round 32-bit integer values (from MSB).
///
/// # Panics
/// Panics if `bytes.len()` is not a multiple of 4.
pub fn round_bytes_int32(bytes: &mut [u8], keepbits: u32) {
    for chunk in bytes.as_chunks_mut::<4>().0 {
        let element = u32::from_ne_bytes(*chunk);
        let rounded = round_bits32(element, keepbits, 32 - element.leading_zeros());
        chunk.copy_from_slice(&u32::to_ne_bytes(rounded));
    }
}

/// Helper to round 64-bit integer values (from MSB).
///
/// # Panics
/// Panics if `bytes.len()` is not a multiple of 8.
pub fn round_bytes_int64(bytes: &mut [u8], keepbits: u32) {
    for chunk in bytes.as_chunks_mut::<8>().0 {
        let element = u64::from_ne_bytes(*chunk);
        let rounded = round_bits64(element, keepbits, 64 - element.leading_zeros());
        chunk.copy_from_slice(&u64::to_ne_bytes(rounded));
    }
}

/// Helper to round 16-bit float values (fixed mantissa bits).
///
/// # Panics
/// Panics if `bytes.len()` is not a multiple of 2.
pub fn round_bytes_float16(bytes: &mut [u8], keepbits: u32, mantissa_bits: u32) {
    for chunk in bytes.as_chunks_mut::<2>().0 {
        let element = u16::from_ne_bytes(*chunk);
        let rounded = round_bits16(element, keepbits, mantissa_bits);
        chunk.copy_from_slice(&u16::to_ne_bytes(rounded));
    }
}

/// Helper to round 32-bit float values (fixed mantissa bits).
///
/// # Panics
/// Panics if `bytes.len()` is not a multiple of 4.
pub fn round_bytes_float32(bytes: &mut [u8], keepbits: u32, mantissa_bits: u32) {
    for chunk in bytes.as_chunks_mut::<4>().0 {
        let element = u32::from_ne_bytes(*chunk);
        let rounded = round_bits32(element, keepbits, mantissa_bits);
        chunk.copy_from_slice(&u32::to_ne_bytes(rounded));
    }
}

/// Helper to round 64-bit float values (fixed mantissa bits).
///
/// # Panics
/// Panics if `bytes.len()` is not a multiple of 8.
pub fn round_bytes_float64(bytes: &mut [u8], keepbits: u32, mantissa_bits: u32) {
    for chunk in bytes.as_chunks_mut::<8>().0 {
        let element = u64::from_ne_bytes(*chunk);
        let rounded = round_bits64(element, keepbits, mantissa_bits);
        chunk.copy_from_slice(&u64::to_ne_bytes(rounded));
    }
}
