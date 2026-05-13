// Source: https://github.com/glium/glium/blob/8d6fd34d9171172928771657fc5c9103107ff978/src/draw_parameters/mod.rs
// License: Apache-2.0
// Note: minimal extract of one top-level `pub fn` from glium/glium@8d6fd34d9171172928771657fc5c9103107ff978 src/draw_parameters/mod.rs (upstream line 530-545, audit-corpus lines 5-20 after the 3-line header + 1 blank-line offset). The function carries a `///` doc block whose single line reads `DEPRECATED. Checks parameters and returns an error if something is wrong.` but does not carry the `#[deprecated]` runtime attribute the Rust deprecation lints honour, so downstream consumers receive no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment") bug shape, the same one cntrdct's batch-3 `sidan-lab/whisky-archive` con_str* family and batch-11 `rusticata/tls-parser` `parse_tls_handshake_*next_protocol` family flag. Unrelated imports, struct/enum definitions, and surrounding draw-parameter helpers are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern C check operate per-fn; tree-sitter parses unresolved type identifiers (`Context`, `DrawParameters`, `DrawError`, `Version`, `Api`) syntactically without requiring resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// DEPRECATED. Checks parameters and returns an error if something is wrong.
pub fn validate(context: &Context, params: &DrawParameters<'_>) -> Result<(), DrawError> {
    if params.depth.range.0 < 0.0 || params.depth.range.0 > 1.0 ||
       params.depth.range.1 < 0.0 || params.depth.range.1 > 1.0
    {
        return Err(DrawError::InvalidDepthRange);
    }

    if !params.draw_primitives && context.get_opengl_version() < &Version(Api::Gl, 3, 0) &&
        !context.get_extensions().gl_ext_transform_feedback
    {
        return Err(DrawError::RasterizerDiscardNotSupported);
    }

    Ok(())
}
