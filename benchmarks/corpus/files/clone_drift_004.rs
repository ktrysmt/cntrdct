// Source: https://github.com/BurntSushi/regex/blob/master/regex-syntax/src/ast/parse.rs
// Note: shape adapted from upstream token-loop family.
// License: MIT OR Apache-2.0
// Note: four near-identical token consumers plus one drifted member exhibit
// the Type-3 with Type-2 partition drift documented in Bettenburg et al.
// (MSR 2009).

fn consume_a_004(bytes: &[u8]) -> u32 {
    let mut acc = 0;
    for b in bytes {
        if *b > 0 {
            acc += *b as u32;
        }
    }
    acc
}

fn consume_b_004(bytes: &[u8]) -> u32 {
    let mut acc = 0;
    for b in bytes {
        if *b > 0 {
            acc += *b as u32;
        }
    }
    acc
}

fn consume_c_004(bytes: &[u8]) -> u32 {
    let mut acc = 0;
    for b in bytes {
        if *b > 0 {
            acc += *b as u32;
        }
    }
    acc
}

fn consume_d_004(bytes: &[u8]) -> u32 {
    let mut acc = 0;
    for b in bytes {
        if *b > 0 {
            acc += *b as u32;
        }
    }
    acc
}

fn consume_drifted_004(bytes: &[u8]) -> u32 {
    let mut acc = 0;
    for b in bytes {
        if *b > 0 {
            acc += *b as u32;
        } else {
            acc = acc.saturating_sub(1);
        }
    }
    acc
}
