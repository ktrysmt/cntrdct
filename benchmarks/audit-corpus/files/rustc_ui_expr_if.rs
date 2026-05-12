// Source: https://github.com/rust-lang/rust/blob/4b0c9d76ae7d387229caea55cfa73c280b08b8a7/tests/ui/reachable/expr_if.rs
// License: MIT OR Apache-2.0
// Note: verbatim extract from rust-lang/rust ui-test fixture for the rustc `unreachable_code` lint, with the file-level `#![deny(unreachable_code)]` attribute stripped because cntrdct's suppression scanner honors any attribute containing the substring `unreachable_code` (src/detectors/unreachable_after_terminator.rs SUPPRESSION_TOKEN)
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]

fn foo() {
    if {return} { //~ ERROR unreachable block in `if`
        println!("Hello, world!");
    }
}

fn bar() {
    if {true} {
        return;
    }
    println!("I am not dead.");
}

fn baz() {
    if {true} {
        return;
    } else {
        return;
    }
    // As the next action to be taken after the if arms, we should
    // report the `println!` as unreachable:
    println!("But I am.");
    //~^ ERROR unreachable statement
}

fn main() { }
