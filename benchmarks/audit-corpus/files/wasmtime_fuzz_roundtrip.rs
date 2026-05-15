// Source: https://github.com/bytecodealliance/wasmtime/blob/63330f11f50f6721a0f4b41ae366bba00a58536d/cranelift/assembler-x64/src/fuzz.rs
// License: Apache-2.0 WITH LLVM-exception
// Note: minimal extract of one top-level `pub fn roundtrip` item from bytecodealliance/wasmtime@63330f11f50f6721a0f4b41ae366bba00a58536d cranelift/assembler-x64/src/fuzz.rs (upstream line 26, corpus line 13 after the 3-line provenance header + 1 blank-line offset). After `docs/spec/comment-code-v0.md` F2 rendering the doc text contains the substring `may fail`, which is one of the six Pattern A trigger phrases enumerated in spec F3; the function signature `(...) ` has unit return so the return type does not contain `Result` / `Option`, satisfying F3's return-type negation — the doc claim of fallibility is not propagated to the caller through the type system. The body acknowledges the failure mode via `panic!`/`assert_eq!`/`unwrap` (the fuzzer infrastructure intentionally panics to express failure), so spec F4 Pattern B does NOT fire — only Pattern A. This is the SECOND Pattern A upstream in the audit corpus, breaking the single-upstream dominance batch 15 boundless introduced — the batch-15 boundless `default_registry` case is the silent-absorb-and-log sub-shape (doc says "may fail", body absorbs via `tracing::warn!` and returns partial), while this wasmtime `roundtrip` case is the documented-panic-on-failure sub-shape (doc says "may fail", body panics via `unwrap` / `assert_eq!`). Both are syntactic Pattern A hits, exercising both sub-shapes on two unrelated upstreams (zkVM executor registry vs. assembler fuzzer infrastructure). SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Take a random assembly instruction and check its encoding and
/// pretty-printing against a known-good disassembler.
///
/// # Panics
///
/// This function panics to express failure as expected by the `arbitrary`
/// fuzzer infrastructure. It may fail during assembly, disassembly, or when
/// comparing the disassembled strings.
pub fn roundtrip(inst: &Inst<FuzzRegs>) {
    // Check that we can actually assemble this instruction.
    let assembled = assemble(inst);
    let expected = disassemble(&assembled, inst);

    // Check that our pretty-printed output matches the known-good output. Trim
    // off the instruction offset first.
    let expected = expected.split_once(' ').unwrap().1;
    let actual = inst.to_string();
    if expected != actual && expected.trim() != fix_up(&actual) {
        println!("> {inst}");
        println!("  debug: {inst:x?}");
        println!("  assembled: {}", pretty_print_hexadecimal(&assembled));
        println!("  expected (capstone): {expected}");
        println!("  actual (to_string):  {actual}");
        assert_eq!(expected, &actual);
    }
}
