// Source: https://github.com/rusticata/tls-parser/blob/6554155918278531370e7d0addbd5d759e3a4cc9/src/tls_handshake.rs
// License: MIT OR Apache-2.0
// Note: minimal extract of two top-level `pub fn` items from rusticata/tls-parser@6554155918278531370e7d0addbd5d759e3a4cc9 src/tls_handshake.rs (upstream lines 845-870, audit-corpus lines 5-30 after the 3-line header + 1 blank-line offset). Both functions carry a `///` doc block whose last line reads `Deprecated in favour of ALPN.` but neither carries the `#[deprecated]` runtime attribute the Rust deprecation lints honour, so downstream consumers receive no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment") bug shape, the same one cntrdct's batch-3 `sidan-lab/whisky-archive` con_str* family flags. Unrelated imports, type definitions, and surrounding handshake-parser functions are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern C check operate per-fn; tree-sitter parses unresolved type identifiers (`IResult`, `TlsNextProtocolContent`, `TlsMessageHandshake`, `length_data`, `be_u8`, `map`) syntactically without requiring resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Parse handshake message contents for NextProtocol
///
/// NextProtocol handshake message, as defined in
/// [draft-agl-tls-nextprotoneg-03](https://tools.ietf.org/html/draft-agl-tls-nextprotoneg-03)
/// Deprecated in favour of ALPN.
pub fn parse_tls_handshake_next_protocol(i: &[u8]) -> IResult<&[u8], TlsNextProtocolContent<'_>> {
    let (i, selected_protocol) = length_data(be_u8)(i)?;
    let (i, padding) = length_data(be_u8)(i)?;
    let next_proto = TlsNextProtocolContent {
        selected_protocol,
        padding,
    };
    Ok((i, next_proto))
}

/// Parse a NextProtocol handshake message
///
/// NextProtocol handshake message, as defined in
/// [draft-agl-tls-nextprotoneg-03](https://tools.ietf.org/html/draft-agl-tls-nextprotoneg-03)
/// Deprecated in favour of ALPN.
pub fn parse_tls_handshake_msg_next_protocol(i: &[u8]) -> IResult<&[u8], TlsMessageHandshake<'_>> {
    map(
        parse_tls_handshake_next_protocol,
        TlsMessageHandshake::NextProtocol,
    )(i)
}
