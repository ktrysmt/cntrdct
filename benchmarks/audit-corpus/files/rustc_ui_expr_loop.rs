// Source: https://github.com/rust-lang/rust/blob/4b0c9d76ae7d387229caea55cfa73c280b08b8a7/tests/ui/reachable/expr_loop.rs
// License: MIT OR Apache-2.0
// Note: verbatim extract from rust-lang/rust ui-test fixture for the rustc `unreachable_code` lint, with the file-level `#![deny(unreachable_code)]` attribute stripped because cntrdct's suppression scanner honors any attribute containing the substring `unreachable_code` (src/detectors/unreachable_after_terminator.rs SUPPRESSION_TOKEN)
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]

fn a() {
    loop { return; }
    println!("I am dead.");
    //~^ ERROR unreachable statement
}

fn b() {
    loop {
        break;
    }
    println!("I am not dead.");
}

fn c() {
    loop { return; }
    println!("I am dead.");
    //~^ ERROR unreachable statement
}

fn d() {
    'outer: loop { loop { break 'outer; } }
    println!("I am not dead.");
}

fn e() {
    loop { 'middle: loop { loop { break 'middle; } } }
    println!("I am dead.");
    //~^ ERROR unreachable statement
}

fn main() { }
