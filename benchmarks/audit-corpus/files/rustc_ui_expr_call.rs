// Source: https://github.com/rust-lang/rust/blob/4b0c9d76ae7d387229caea55cfa73c280b08b8a7/tests/ui/reachable/expr_call.rs
// License: MIT OR Apache-2.0
// Note: verbatim extract from rust-lang/rust ui-test fixture for the rustc `unreachable_code` lint, with the file-level `#![deny(unreachable_code)]` attribute stripped because cntrdct's suppression scanner honors any attribute containing the substring `unreachable_code` (src/detectors/unreachable_after_terminator.rs SUPPRESSION_TOKEN)
#![feature(never_type)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]

fn foo(x: !, y: usize) { }

fn bar(x: !) { }

fn a() {
    // the `22` is unreachable:
    foo(return, 22); //~ ERROR unreachable
}

fn b() {
    // the call is unreachable:
    bar(return); //~ ERROR unreachable
}

fn main() { }
