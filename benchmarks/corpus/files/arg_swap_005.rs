// Source: signature pattern adapted from
// https://github.com/tokio-rs/tokio/blob/master/tokio/src/io/util/copy.rs (reader/writer order)
// License: MIT
// Note: the call at line 14 swaps (reader, writer) to exhibit the arg-swap pattern
// documented in Rice et al. (ICSE 2017). Files in this corpus do not need to
// type-check; only the parse tree matters for the detector.

fn pipe_bytes_005(reader: &[u8], writer: &mut Vec<u8>) -> usize {
    writer.extend_from_slice(reader);
    reader.len()
}

fn entry_005() {
    let reader = 1;
    let writer = 2;
    let _ = pipe_bytes_005(writer, reader);
}
