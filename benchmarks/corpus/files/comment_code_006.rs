// Source: signature pattern adapted from
// https://github.com/tokio-rs/tokio/blob/master/tokio/src/sync/mutex.rs (try_lock style)
// License: MIT
// Note: Pattern B — the doc says "panics on contention" but the body has no
// panicking construct. Drift documented in Tan et al., iComment SOSP 2007.

/// Panics on contention; the contention check is a no-op in this prototype.
fn try_acquire_006(state: u32) -> u32 {
    state.saturating_add(1)
}
