# IR v0 spec

Status: draft, post-R-0-review revision (2026-05-24). Blockers raised
in the 4-axis parallel review absorbed.

## Background

cntrdct v0.5.x parses every analysed file twice per cross-cutting
detector run: once per detector, once per language arm inside the
detector. The `ParsedFile` carried into `Detector::detect` is a
`{path, language, source}` triple; each detector then constructs its
own tree-sitter parser, walks the AST with a per-language helper
(`scan_rust` / `scan_python`), and renormalises the same shape (call
sites, blocks, comments, terminators, function definitions) that
every other cross-cutting detector also computes. `REBUILD.md` goal
G1 makes this duplication explicit and proposes a language-agnostic
Intermediate Representation (IR) that absorbs the per-language scan
helpers into a single conversion pass shared by every cross-cutting
detector.

This spec defines the IR node shape, the conversion contract from
tree-sitter to IR per `ParserProvider`, the `Detector` trait
migration from `ParsedFile` to `IrFile`, the escape hatch by which
language-specific detectors (`src/detectors/lang/`) retain raw
tree-sitter access, and the test plan that gates R-1 implementation
on byte-identical findings against the existing audit-corpus +
wild-corpus baselines.

The IR layer is internal to Layer 1. P3 is preserved (no socket-
opening code path is introduced); P1 / P4 / P5 are unchanged because
citations, priors, and SARIF severity mapping all live above IR.

## Scope

In:

- The IR node shape (`IrFile`, `IrFn`, `IrBlock`, `IrStmt` /
  `IrStmtKind`, `IrCallSite`, `IrComment`, `IrPath`, `IrExpr`,
  `IrLiteral`, `IrTerminator`, `IrParam`, `IrDecorator`, `NodeRef`,
  plus the `DivergentKind` / `BranchMergeKind` /
  `ConstantBranchKind` / `HoistedItemKind` / `ParamKind`
  classifier enums) and its invariants.
- The conversion contract: `ParserProvider::to_ir(tree, source, path)
  -> Result<IrFile, IrConvertError>`, partial-parse handling, the
  unknown-node fallback, the `tree-sitter::Tree::root_node().has_error()`
  carry-through via `IrFile.parse_recovered`, and the production-scan
  runtime behaviour for every `IrConvertError` variant.
- The `Detector` trait migration: `&DetectContext<'a>` switches its
  internal `[ParsedFile]` slice to `[IrFile]`. `cntrdct` (the single
  crate) bumps from 0.5.x to 0.6.0. No `ParsedFile::ir()` compatibility
  shim is provided. The five cross-cutting detector specs
  (`arg-swap-v0.md`, `clone-drift-v0.md`, `comment-code-v0.md`,
  `unreachable-after-terminator-v0.md`, `pr-miner-v0.md`) and
  `lsp-v0.md` have their `ParsedFile` references overridden by this
  spec; the R-1 commit applies a textual sweep across those files.
- The language-specific escape hatch: `IrFile::raw_tree()` (lazy
  reparse method per R1) and `IrFile.source` so detectors under
  `src/detectors/lang/` can drop into the raw AST.
- The location preservation invariant: every IR `Location` line /
  column equals the source tree-sitter node's `start_position()` and
  `end_position()` plus 1, byte-identical to the v0.5.x mapping.
  `Location.start_byte` / `end_byte` additionally pin the byte
  range against `Node.start_byte()` / `end_byte()` for detectors
  that consume raw source slices.
- The LSP non-regression requirement: `cntrdct-lsp` keeps its
  last-successful-IR cache so `didChange` traffic does not blank
  diagnostics while typing.
- The R-1 test plan: pinning tests against audit-corpus +
  wild-corpus, IR golden fixtures per cross-cutting detector under
  `tests/fixtures/ir/`, `IrConvertError` variant-per-fixture coverage,
  and a recall-regression check via
  `cntrdct calibrate --audit-recall benchmarks/audit-corpus`.

Out:

- IR is not a complete AST. It models only the structure the five
  cross-cutting detectors consume (call sites, blocks with terminator
  classification, comments, functions with parameter / decorator
  metadata, normalised token sequences, plus the `with` / hoisted-item
  shapes the existing detectors rely on). Anything outside that
  surface either parses to an `IrStmtKind::Other` / `IrExpr::Other`
  placeholder (carrying the source node kind for discrimination) or
  is retrieved by language-specific detectors via `IrFile::raw_tree()`.
- IR-to-source pretty-printing.
- IR persistence or cross-run caching.
- New `Language` variants beyond `Rust` / `Python` (R-2 / R-3 cover
  TypeScript / Go separately).
- Cross-cutting detector additions whose semantics are not already
  in cntrdct v0.5.x. R-1 is a strict refactor: same findings, same
  citations, same SARIF, IR-mediated computation path.

## Glossary

- IR — Intermediate Representation. The set of types under `src/ir.rs`
  this spec defines.
- Cross-cutting detector — a detector whose concept transfers across
  languages: `arg-swap`, `clone-drift`, `comment-code`,
  `unreachable-after-terminator`, `pr-miner`. Consumes IR.
- Language-specific detector — a detector whose concept is bound to
  one language's syntax: `config-interaction` (Rust `#[cfg]`),
  the planned R-5 Python `except`-reachability detector. Reads
  tree-sitter ASTs via `IrFile::raw_tree()` (lazy reparse).
- `ParsedFile` — the v0.5.x detector input type. Retired by this
  spec; `IrFile` replaces it.

## Functional requirements

### F1 — IR node definitions

The minimum node set required by the five cross-cutting detectors.
Every field name and type below is binding for R-1; additions land
through a new spec section if a future cross-cutting detector needs
them.

`IrStmt` is a struct with a `kind: IrStmtKind` enum payload, an
`attributes: Vec<IrDecorator>` carrier for preceding cfg /
decorator nodes, and a `location: Location`. The struct-with-enum
shape (rather than a bare enum with location inside every variant)
lets `unreachable-after-terminator` walk per-statement cfg gating
uniformly and avoids the F5 escape-hatch leak that an
`IrStmt::Other`-only model would force.

```rust
pub struct IrFile {
    pub path: PathBuf,
    pub language: Language,
    pub source: Arc<str>,
    pub fns: Vec<IrFn>,
    pub top_level_comments: Vec<IrComment>,
    /// True when tree-sitter's `root_node().has_error()` was true.
    /// Detectors currently use `if root.has_error() { return None }`
    /// as a hard skip; the carry-through preserves that contract.
    pub parse_recovered: bool,
    // No `raw_tree` field. Per R1 (lazy reparse mitigation), the
    // tree-sitter tree is produced on demand via `IrFile::raw_tree()`
    // below — not stored as a field — so peak RSS scales with the
    // number of concurrent detector tasks rather than the corpus size.
}

impl IrFile {
    /// Parse `self.source` with the per-language tree-sitter grammar
    /// and return a fresh `Arc<SyncTree>`. Each call produces an
    /// independent tree that drops when the caller releases the Arc.
    /// Cross-cutting detectors must not call this method; language-
    /// specific detectors (F5 escape hatch) reparse on demand.
    pub fn raw_tree(&self) -> Arc<SyncTree> {
        // implementation: tree_sitter::Parser::new(),
        // set_language(parser_for(self.language).ts_language()),
        // parser.parse(&self.source[..], None), Arc::new(SyncTree::new(tree)).
        // The reparse cannot fail under the IR contract: the same
        // source already parsed successfully once at to_ir time.
        unimplemented!("spec sketch — see src/ir.rs")
    }

    /// Resolve a `NodeRef` against a freshly-parsed tree and pass the
    /// matching node to `f`. Returns `None` when the ref was produced
    /// against a different `IrFile` (the caller misused the API). The
    /// closure form lets the spec hide the lazy reparse: the tree
    /// drops when `f` returns.
    pub fn resolve_with<R>(
        &self,
        node_ref: &NodeRef,
        f: impl FnOnce(tree_sitter::Node<'_>) -> R,
    ) -> Option<R> {
        let raw = self.raw_tree();
        find_node_with_range(raw.root_node(), node_ref.range).map(f)
    }
}

pub struct IrFn {
    pub name: String,
    pub params: Vec<IrParam>,
    pub body: IrBlock,
    /// Raw return-type text per language: Rust's `-> T` suffix, the
    /// Python `function_definition` `return_type` field, or `None`
    /// when absent. Used by `comment-code` Pattern A to suppress
    /// "claims Result/Option but return type already says so".
    /// Stored as a string so the IR doesn't model type expressions.
    pub return_type_text: Option<String>,
    /// Decorators (Python) or outer attributes (Rust) attached to
    /// this function in source order. Used by `comment-code`
    /// Pattern C / py-deprecated.
    pub decorators: Vec<IrDecorator>,
    /// True when this function is a method inside a `class_definition`
    /// (Python) or an `impl_item` (Rust). `arg-swap` uses this flag
    /// to drop the leading `self` / `cls` parameter before arity
    /// checks.
    pub is_method: bool,
    /// Leading doc text in canonical, prefix-stripped form. Rust:
    /// `///` lines joined with `\n`. Python: the first
    /// `expression_statement` of `body` when it is a bare string,
    /// quotes stripped. `None` when no leading doc is present.
    /// Consumed by `comment-code`. Distinct from
    /// `IrFile.top_level_comments` / `IrFile`-internal `IrComment`s:
    /// `leading_doc` is the pre-rendered text the detector wants,
    /// while `IrComment` is the structural node retained for
    /// detectors that need delimiter / kind information.
    pub leading_doc: Option<String>,
    /// Normalised token sequence rooted at the whole function item
    /// (Rust `function_item`, Python `function_definition`), the leaf
    /// kinds enumerated in `NormalisedToken`; comment nodes excluded.
    /// Consumed by `clone-drift`'s function-level clustering. Rooting
    /// at the function item — not the body block — preserves the
    /// v0.5.x `walk_normalize_*(function_item)` sequence byte-for-byte
    /// so the signature prefix participates in the n-gram set.
    /// Populated once per function (R2).
    pub normalised_tokens: Vec<NormalisedToken>,
    pub location: Location,
}

pub struct IrParam {
    pub name: String,
    pub kind: ParamKind,
    /// Default-value literal (trimmed source text) where the language
    /// admits one and the parameter declares it (Python `a=expr` /
    /// `a: T = expr`, TypeScript `a = expr`); `None` for no default and
    /// for Rust / Go (no default-parameter syntax). Added for the Layer 0
    /// candidate predicate (p3-amendment-v0.md review M6). Serialised with
    /// `skip_serializing_if = Option::is_none` so the F6 T4 golden wire
    /// shape stays byte-identical for the common no-default case.
    pub default: Option<String>,
    pub location: Location,
}

pub enum ParamKind {
    /// Plain positional parameter the detector can reason about.
    Plain,
    /// Implicit receiver (Python `self` / `cls` first parameter).
    /// `arg-swap` drops receiver params before arity checks.
    Receiver,
    /// Parameter shape the cross-cutting layer cannot model:
    /// Python `*args` / `**kwargs` / `/` / `*` separators, Rust
    /// `self` patterns the converter does not unwrap, anything the
    /// converter declines to fold into `Plain` or `Receiver`.
    /// `arg-swap` rejects the entire function definition when any
    /// param is `Unsupported`, matching the v0.5.x conservatism.
    Unsupported,
}

pub struct IrBlock {
    pub statements: Vec<IrStmt>,
    /// Terminator classification for the block as a whole, computed
    /// by the converter via F4d-style branch-merge analysis. `Some`
    /// iff every reachable path through the block ends in a
    /// divergent expression. `unreachable-after-terminator` consumes
    /// this when classifying a block-shaped tail expression.
    pub terminator: Option<IrTerminator>,
    /// Count of normalised tokens this block would produce when walked
    /// in isolation (block-rooted `walk_normalize_*`). `clone-drift`'s
    /// F2b intra-fn `if`-same-then-else gate reads the consequence
    /// block's count for its size threshold and finding message. Only
    /// the count is stored — not the vector — so per-block memory stays
    /// O(1) rather than the O(tokens × nesting-depth) a per-block
    /// vector would incur (R2). The function-level token sequence lives
    /// on `IrFn.normalised_tokens`.
    pub normalised_token_count: usize,
    pub location: Location,
}

pub struct IrStmt {
    pub kind: IrStmtKind,
    /// Preceding attribute / decorator nodes that bind to this
    /// statement under its enclosing block. Rust: `#[cfg(...)]`
    /// `attribute_item` nodes immediately preceding the statement
    /// in the same block (the v0.5.x `is_cfg_gated_statement`
    /// scan target). Python: the empty vector — Python attaches
    /// decorators to function / class definitions, not arbitrary
    /// statements; per-statement Python attributes are vacuously
    /// absent. Empty for statements with no preceding attributes.
    pub attributes: Vec<IrDecorator>,
    pub location: Location,
}

pub enum IrStmtKind {
    /// A function call sitting as a statement (Rust
    /// `expression_statement` wrapping a `call_expression`, Python
    /// `expression_statement` wrapping a `call`).
    Call(IrCallSite),
    /// Rust `let <pat> = <value>;`. `value` is the initialiser
    /// expression (`None` for `let x;`). The RHS is materialised so a
    /// cross-cutting detector reaches call sites and nested terminators
    /// inside it (`let x = { return ...; };`, the
    /// `rustc_ui_expr_return.rs` audit shape) without dropping to
    /// `raw_tree()`. The binding pattern is not modelled.
    Let { value: Option<IrExpr> },
    /// Python `<lhs> = <value>` (plain `assignment`). `value` is the
    /// RHS expression (`None` for an annotation-only `x: int`). Mirrors
    /// `Let` for the Python assignment-wrapped statement shape
    /// (`_ = copy(src, dst)`) so the call is visible to an IR walk.
    /// Augmented assignments (`x += 1`) stay `Other`.
    Assign { value: Option<IrExpr> },
    Return(Option<IrExpr>),
    Raise(Option<IrExpr>),
    Break(Option<IrLabel>),
    Continue(Option<IrLabel>),
    /// `assert <cond>` (Python) or `assert!(<cond>)` (Rust). The
    /// inner `IrExpr` carries the condition so the
    /// `assert False` / `assert!(false)` shape is recognisable
    /// without further parsing.
    Assert(IrExpr),
    /// Macro / call shapes whose semantics are divergent. The
    /// `kind` enum names the canonical terminator.
    DivergentCall { kind: DivergentKind, args: Vec<IrExpr> },
    If(IrIfStmt),
    While(IrWhileStmt),
    Loop(IrLoopStmt),
    /// `for <pat> in <iterable>` (Rust `for_expression`, Python
    /// `for_statement`). The iterable expression and the loop body are
    /// both materialised so calls in either position are reachable
    /// from an IR walk (the `rarfile_set_attrs.py` audit shape places
    /// the arg-swap call inside a `for` body). The loop variable
    /// pattern is not modelled.
    For(IrForStmt),
    Match(IrMatchStmt),
    /// `with <ctx> as <name>: <body>` (Python). `pr-miner` F4e-i
    /// uses the surrounding `With` to suppress a synthesised
    /// `close` rule when the call's owner is already managed by a
    /// context manager.
    With(IrWithStmt),
    /// Python `try: <body> except ...: <handler> [else] [finally]`.
    /// Every sub-block is materialised so calls under any clause are
    /// reachable from an IR walk. The exception-type / binding shapes
    /// in the `except` clauses are not modelled.
    Try(IrTryStmt),
    /// A nested item declaration that the compiler hoists out of
    /// statement order. The variant exists so
    /// `unreachable-after-terminator` F4c can skip these without
    /// dropping to `raw_tree()`. Variant covers Rust
    /// `function_item` / `mod_item` / `use_declaration` / etc.;
    /// Python has no direct analogue (function / class definitions
    /// in Python are statements with runtime effect and stay as
    /// the appropriate `IrStmtKind`).
    HoistedItem { kind: HoistedItemKind, node_ref: NodeRef },
    /// Any statement shape the converter does not model. The
    /// `node_kind` discriminator is the tree-sitter `Node::kind()`
    /// string so a cross-cutting detector can filter without going
    /// through `raw_tree()` (e.g. `unreachable-after-terminator` may
    /// want to skip `empty_statement` without recovering the raw
    /// node). The raw tree-sitter node remains recoverable via
    /// `IrFile::resolve_with(&node_ref, ..)` for language-specific
    /// detectors.
    Other { node_kind: &'static str, node_ref: NodeRef },
}

pub struct IrIfStmt {
    pub condition: IrExpr,
    pub consequence: IrBlock,
    pub alternative: Option<IrBlock>,
    /// Pre-computed branch-merge terminator. `Some(BranchMerge { ... })`
    /// when every branch (consequence + alternative) ends in a
    /// divergent expression. `unreachable-after-terminator` reads
    /// this instead of recursing into the branches at scan time.
    pub terminator: Option<IrTerminator>,
    pub location: Location,
}

pub struct IrWhileStmt {
    pub condition: IrExpr,
    pub body: IrBlock,
    pub location: Location,
}

pub struct IrLoopStmt {
    /// True when the body contains a `break` whose target is this
    /// loop. Computed by the converter via the same labelled-break
    /// analysis the Rust detector currently runs at scan time.
    /// `unreachable-after-terminator` uses this to classify
    /// `loop { ... }` as divergent when no break targets it.
    pub label: Option<IrLabel>,
    pub body: IrBlock,
    pub has_break_to_self: bool,
    pub location: Location,
}

pub struct IrMatchStmt {
    pub scrutinee: IrExpr,
    pub arms: Vec<IrMatchArm>,
    /// Pre-computed branch-merge terminator. `Some(BranchMerge { ... })`
    /// when every arm body ends in a divergent expression.
    pub terminator: Option<IrTerminator>,
    pub location: Location,
}

pub struct IrMatchArm {
    pub body: IrExpr,
    pub location: Location,
}

pub struct IrWithStmt {
    /// Context-manager expressions. Multiple managers
    /// (`with a as x, b as y:`) appear in source order.
    pub context_managers: Vec<IrExpr>,
    pub body: IrBlock,
    pub location: Location,
}

pub struct IrForStmt {
    /// Iterable expression (Rust `for_expression.value`, Python
    /// `for_statement.right`).
    pub iterable: IrExpr,
    pub body: IrBlock,
    pub location: Location,
}

pub struct IrTryStmt {
    /// `try:` body block.
    pub body: IrBlock,
    /// `except ...:` handler bodies in source order. The
    /// exception-type expression and the bound name are not modelled;
    /// only the handler block is retained for call walking.
    pub handlers: Vec<IrBlock>,
    /// `else:` body, if present.
    pub orelse: Option<IrBlock>,
    /// `finally:` body, if present.
    pub finalbody: Option<IrBlock>,
    pub location: Location,
}

pub struct IrCallSite {
    /// Resolved callee path. Bare identifiers populate
    /// `segments = ["foo"]`; dotted / scoped paths populate
    /// `segments = ["foo", "bar"]`. Python `self.method` /
    /// `cls.method` populate `receiver = Some("self" | "cls")` and
    /// `segments = ["method"]` — `arg-swap`'s F3b shape.
    /// `pr-miner` F4e-ii reads the full receiver chain to
    /// recognise idiomatic patterns like `logger.info(...)`.
    pub callee: IrPath,
    pub args: Vec<IrExpr>,
    pub location: Location,
}

pub struct IrPath {
    /// Receiver chain (zero or more segments before the final name).
    /// Python `self.method` produces `receiver = ["self"]` and
    /// `segments = ["method"]`. Python `obj.attr.method` produces
    /// `receiver = ["obj", "attr"]` and `segments = ["method"]`.
    /// Rust `foo::bar::baz` produces an empty receiver and
    /// `segments = ["foo", "bar", "baz"]`.
    pub receiver: Vec<String>,
    pub segments: Vec<String>,
    /// Raw source text of the callee, preserved for detectors that
    /// want to display the original spelling (`foo::bar::baz` vs
    /// `foo.bar.baz`).
    pub raw: String,
}

/// An expression with its source location. Every expression — not
/// just `Other` — carries a `Location` so
/// `unreachable-after-terminator` can report F4d-ii / F4d-iii / F4d-iv
/// and F4e finding endpoints (divergent call argument, divergent
/// return-or-break value, divergent / constant `if`-`while` condition)
/// at the exact span the v0.5.x raw-tree walk used, without dropping
/// to `raw_tree()` (R-1.c'' Path b). For `Call` / `Block` / `If` /
/// `Match` / `Loop` the `location` equals the boxed inner node's
/// location (same tree-sitter node); the duplication keeps every
/// `IrExpr` uniformly self-describing.
pub struct IrExpr {
    pub kind: IrExprKind,
    pub location: Location,
}

pub enum IrExprKind {
    Ident(String),
    Path(IrPath),
    Literal(IrLiteral),
    Call(Box<IrCallSite>),
    /// Value-carrying `return <expr>`; `Return(None)` is the
    /// valueless form (Rust `return;`, Python bare `return`).
    Return(Option<Box<IrExpr>>),
    /// Value-carrying `raise <expr>`; `Raise(None)` is the
    /// re-raise form (Python bare `raise` inside `except`).
    Raise(Option<Box<IrExpr>>),
    Break(Option<IrLabel>),
    Continue(Option<IrLabel>),
    Block(Box<IrBlock>),
    If(Box<IrIfStmt>),
    Match(Box<IrMatchStmt>),
    Loop(Box<IrLoopStmt>),
    /// Macro / function call whose semantics are divergent.
    /// Mirrors `IrStmtKind::DivergentCall` for the expression
    /// position (e.g. `panic!()` used as a bare expression).
    DivergentCall { kind: DivergentKind, args: Vec<IrExpr> },
    /// Any expression shape the converter does not model.
    Other { node_kind: &'static str, node_ref: NodeRef },
}

pub enum IrLiteral {
    Bool(bool),
    /// Decimal integer value when the literal parses cleanly via
    /// `i128::from_str_radix(_, 10)`. Hex / octal / binary literals
    /// stay `None` (F4e v0 contract).
    Int(Option<i128>),
    Float,
    /// `is_empty` is true when the string literal carries no
    /// content between its delimiters. Used by F4e Python
    /// constant-condition classification (empty string is falsy).
    String { is_empty: bool },
    Char,
    /// Python `None`.
    None,
}

pub enum NormalisedToken {
    /// Structural AST node kind (e.g. "block", "if_expression",
    /// "function_definition"). The converter passes the
    /// tree-sitter kind through verbatim so the existing
    /// `walk_normalize_*` byte-for-byte output is preserved.
    /// `&'static str` is sound because `tree_sitter::Node::kind()`
    /// itself returns `&'static str` for every grammar cntrdct
    /// links against.
    Kind(&'static str),
    Ident,
    LitInt,
    LitFloat,
    LitStr,
    LitChar,
    LitBool,
}

pub struct IrComment {
    pub kind: IrCommentKind,
    /// Rendered text with the comment delimiter stripped. Rust:
    /// `/// foo` -> `"foo"`. Python docstring: `"""foo"""` -> `"foo"`.
    /// Multi-line comments join with `\n`.
    pub text: String,
    pub target: Option<NodeRef>,
    pub location: Location,
}

pub enum IrCommentKind {
    RustDocLine,
    RustDocBlock,
    RustLine,
    RustBlock,
    PythonComment,
    PythonDocstring,
}

pub struct IrDecorator {
    /// Raw decorator / attribute text. Rust: `#[deprecated]`,
    /// `#[cfg(unix)]`. Python: `@deprecated`, `@functools.cache`.
    pub raw: String,
    /// Dotted name path. Rust: `["deprecated"]`. Python:
    /// `["warnings", "deprecated"]`. The last segment is what
    /// `comment-code` Pattern C and py-deprecated match against.
    pub name_path: Vec<String>,
    pub location: Location,
}

pub enum IrTerminator {
    Return,
    Raise,
    Break,
    Continue,
    /// Macro / function call whose semantics are divergent.
    DivergentCall { kind: DivergentKind },
    /// `assert false` / `assert!(false)` / Python `assert False`.
    AssertFalse,
    /// `loop { ... }` with no break-target.
    LoopNoBreak,
    /// `if / match` whose every branch ends in a divergent
    /// expression.
    BranchMerge { kind: BranchMergeKind },
    /// F4e Python constant-false `while`. Stored as a terminator
    /// because the body's first statement is unreachable.
    ConstantFalseWhile,
    /// F4e Python `if False:` consequence (or `if True: ... else:`
    /// alternative). The carve-out classifier (type-checking
    /// import guard, generator-marker idiom) runs in the detector,
    /// not the converter.
    ConstantBranchUnreachable { kind: ConstantBranchKind },
}

/// Canonical names for divergent macro / function calls.
/// Replaces the v0 draft's `kind: &'static str` strings so
/// `match` arms get exhaustiveness checking and a new variant
/// addition is a compile-time event at every consumer.
pub enum DivergentKind {
    /// Rust `panic!()`.
    Panic,
    /// Rust `unreachable!()`.
    Unreachable,
    /// Rust `todo!()`.
    Todo,
    /// Rust `unimplemented!()`.
    Unimplemented,
    /// Rust `abort!()` family.
    Abort,
    /// Rust `exit!()` family.
    Exit,
    /// Python `sys.exit(...)`.
    SysExit,
    /// Python `sys.abort(...)`.
    SysAbort,
    /// Python `os._exit(...)`.
    OsExit,
    /// Python builtin `exit(...)` (interactive shell).
    ExitBuiltin,
    /// Python builtin `quit(...)` (interactive shell).
    QuitBuiltin,
}

pub enum BranchMergeKind {
    IfBranchesDiverge,
    MatchArmsDiverge,
}

pub enum ConstantBranchKind {
    /// `if False:` consequence.
    ConstantFalseIf,
    /// `if True: ... else:` alternative.
    ConstantTrueIfElse,
}

pub enum HoistedItemKind {
    /// Rust `function_item` or `function_signature_item` declared
    /// inside another block.
    Function,
    /// Rust `mod_item` / `foreign_mod_item`.
    Mod,
    /// Rust `struct_item` / `union_item` / `enum_item` / `type_item`.
    Type,
    /// Rust `const_item` / `static_item`.
    Const,
    /// Rust `trait_item` / `impl_item` / `associated_type`.
    Trait,
    /// Rust `use_declaration` / `extern_crate_declaration`.
    Use,
    /// Rust `macro_definition`.
    Macro,
}

pub enum IrLabel {
    Named(String),
    Unlabelled,
}

/// Opaque reference into the tree returned by `IrFile::raw_tree()`.
/// The `range` field is sufficient for `IrFile::resolve_with` to
/// walk a freshly-parsed tree and recover the matching node; tree
/// identity is implicit (the `NodeRef` is only meaningful when
/// paired with the `IrFile` it was created from). Crossing the two
/// — using a `NodeRef` from one `IrFile` against another — is a
/// programmer error and returns `None` from `IrFile::resolve_with`.
pub struct NodeRef {
    pub range: tree_sitter::Range,
}

pub struct Location {
    pub file: PathBuf,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    /// Byte offsets into `IrFile.source`. Pinned against
    /// `tree_sitter::Node::start_byte()` / `end_byte()` by the
    /// F3 invariant. Detectors that need raw source slices
    /// (`clone-drift` F2b intra-fn if-branch source-text equality,
    /// `comment-code` Pattern B body-marker substring) read
    /// `&ir_file.source[loc.start_byte as usize .. loc.end_byte as usize]`.
    pub start_byte: u32,
    pub end_byte: u32,
}
```

#### Detector traceability

Each cross-cutting detector consumes a documented subset of IR.
This table is normative — R-1 implementation MUST cover every
mapping; conversely, no IR field is added in R-0 without a consumer.

| Detector | IR fields consumed |
|---|---|
| `arg-swap` | DEFINITION extraction only: `IrFn.{name, params, is_method, location}`, `IrParam.{name, kind}` (`Receiver` drop, `Unsupported` whole-fn reject). CALL-SITE enumeration uses `IrFile::raw_tree()` (Pattern-B escape hatch, §F5), NOT IR — reverted 2026-06-03 after the IR-only walk silently dropped calls nested in `IrExpr::Other` shapes (comprehensions, closures, binary, f-strings); full call-set enumeration is not losslessly representable in IR, same rationale as `pr-miner`. See `arg-swap-v0.md` §F3. |
| `clone-drift` | `IrFn.{normalised_tokens, is_method, location}` (function-level clustering, top-level `!is_method` only), `IrStmtKind::If` + `IrExpr::If` walk, `IrIfStmt.{consequence, alternative, location}`, `IrBlock.{normalised_token_count, location}` (F2b consequence size gate) and `Location.{start_byte, end_byte}` for F2b intra-fn if-branch source-text equality against `IrFile.source` |
| `comment-code` | `IrFn.{return_type_text, decorators, leading_doc, body, location}`, `IrBlock.statements`, `IrStmtKind::{Raise, Return, Call}`, `IrDecorator.{name_path, raw}`, `IrComment.{kind, text, target}`, `Location.{start_byte, end_byte}` for Pattern B body-marker substring against `IrFile.source` |
| `unreachable-after-terminator` | `IrBlock.{statements, terminator, location}`, `IrStmt.{kind, attributes, location}`, `IrStmtKind::{Return, Raise, Break, Continue, Assert, DivergentCall, If, While, Loop, Match, HoistedItem}`, `IrTerminator`, `IrIfStmt.{condition, consequence, alternative, terminator}`, `IrMatchStmt.terminator`, `IrWhileStmt.{condition, body}`, `IrLoopStmt.{has_break_to_self, body}`, `IrExpr::Literal` (F4e), `IrStmt.attributes` (F4b per-statement cfg-gated suppression), `IrStmtKind::{For, Try, Let, Assign}` + `IrForStmt.body` / `IrTryStmt.{body, handlers, orelse, finalbody}` (post-terminator analysis inside nested for/try blocks) + `Let`+`Assign` `.value` (Rust nested `let x = { return ...; }` terminator reachability — the `rustc_ui_expr_return.rs` audit shape) |
| `pr-miner` | `IrFn.{name, body, location}`, `IrBlock.statements`, `IrStmtKind::{Call, With, For, Try, Let, Assign}`, `IrWithStmt.{context_managers, body}` (F4e-i), `IrForStmt.{iterable, body}` / `IrTryStmt.{body, handlers, orelse, finalbody}` / `Let`+`Assign` `.value` (full-body call enumeration through nested statement bodies + RHS), `IrCallSite.{callee, args, location}`, `IrPath.receiver` (F4e-ii attribute receiver chain) |

### F2 — Conversion contract

Each `ParserProvider` gains:

```rust
pub trait ParserProvider: Send + Sync {
    fn language(&self) -> Language;
    fn ts_language(&self) -> tree_sitter::Language;
    fn to_ir(
        &self,
        tree: tree_sitter::Tree,
        source: Arc<str>,
        path: PathBuf,
    ) -> Result<IrFile, IrConvertError>;
}
```

Calling convention:

- The caller parses with `tree_sitter::Parser::set_language` +
  `parser.parse(source, None)` and hands the resulting `Tree` to
  `to_ir`. The converter walks the tree to build IR but does NOT
  retain it on `IrFile` — the local tree drops when `to_ir`
  returns. Language-specific detectors recover the tree on demand
  via `IrFile::raw_tree()` (R1 lazy reparse).
- `source` is shared as `Arc<str>` so it can be referenced from
  every IR node without cloning. `Arc<str>` (single allocation) is
  preferred over `Arc<String>` (double allocation) because the
  source is read-only after parsing.
- `path` is the source file's filesystem path, copied into IR nodes'
  `Location.file` field.
- `to_ir` is total over the supplied tree's recognised shapes: it
  never fails on an unrecognised tree-sitter node kind. Unknown
  statement shapes become `IrStmtKind::Other { node_kind, node_ref }`;
  unknown expression shapes become `IrExpr::Other { node_kind, node_ref }`.
  The `node_kind` discriminator lets cross-cutting detectors filter
  by shape without dropping to `raw_tree()`; the `NodeRef` lets a
  language-specific detector recover the raw node when needed (via
  `IrFile::resolve_with`, which itself reparses internally).
- Transparent expression wrappers are unwrapped to their inner
  expression rather than materialised as a distinct node so the
  calls / terminators they carry stay reachable from an IR-only walk:
  Rust / Python `parenthesized_expression` and Python `await`. (No
  cross-cutting detector reasons about parenthesisation or `await`
  itself; both only matter as carriers of an inner call site, so the
  wrapper is dropped.) Wrappers IR does not yet unwrap — Python
  `await`-free shapes such as `binary_operator`, `boolean_operator`,
  `subscript`, `unary_operator`, and the Rust equivalents — still land
  in `IrExpr::Other`; calls nested inside those remain the migrating
  detector's concern per R-1.c''.
- Comment nodes (Rust `line_comment` / `block_comment`,
  Python `comment`) are filtered out of `IrFn.normalised_tokens`,
  the `IrBlock.normalised_token_count` walk, and `IrBlock.statements`
  to preserve the v0.5.x normalisation output byte-identically.
- `parse_recovered` is set from `tree.root_node().has_error()` and
  preserved on `IrFile`. The five cross-cutting detectors currently
  skip files where `has_error()` is true; in R-1 they continue to
  do so by checking `IrFile.parse_recovered` and returning early
  for that file.

```rust
pub enum IrConvertError {
    /// The parser failed to set the requested tree-sitter language.
    /// Surfaced by `parser_for(lang).ts_language()` mismatching the
    /// `tree`'s language. Indicates a build / dependency bug, not a
    /// source-file problem.
    LanguageMismatch { expected: Language, actual: String },
    /// Source is empty or whitespace-only. The converter returns
    /// this rather than producing an `IrFile` with zero functions
    /// so the caller can distinguish "file deliberately blank" from
    /// "tree-sitter saw no top-level items".
    EmptySource,
    /// Internal converter invariant failure — e.g. a tree-sitter
    /// node we expected to have a `child_by_field_name("name")`
    /// did not. Carries the offending node kind for diagnosis.
    /// In R-1 these are programmer errors; a regression test fails
    /// for any fixture that triggers one.
    StructuralInvariant { kind: &'static str, message: String },
}
```

Partial parsing rule: when `tree.root_node().has_error()` is true,
the converter still walks the tree and produces an `IrFile` with
whatever structure was recoverable, but sets `parse_recovered = true`.
Cross-cutting detectors gate on `parse_recovered` to preserve the
current v0.5.x "skip files with parse errors" behaviour. R-5 (Python
F4f) may later opt into recovered files; the gating decision is the
detector's, not the converter's.

Production-scan runtime behaviour for each `IrConvertError` variant
(applies to `cntrdct scan`, `cntrdct calibrate`, `cntrdct eval`, and
`cntrdct-lsp`'s `scan_buffer` — anywhere `to_ir` is called):

- `EmptySource`: the file is silently skipped. Empty files are not
  pathological; they cannot produce findings. No log entry.
- `LanguageMismatch`: this is a programmer error in the cntrdct
  build itself (the file walker selected one `Language` but
  `parser_for(lang)` returned a provider for a different
  `ts_language`). The caller logs at `tracing::error!` level with
  the file path, skips the file, and continues the scan. The
  process does NOT terminate — terminating turns a build bug into a
  CI red on every consumer; the log surfaces it for diagnosis
  without losing the rest of the corpus.
- `StructuralInvariant`: a converter assertion failed on a real
  source file (the fixture suite should have caught it in
  development; production occurrence indicates a gap). The caller
  logs at `tracing::warn!` level with the file path and the
  `{kind, message}` payload, skips the file, and continues. SARIF
  output does NOT include a synthetic finding for the skipped
  file — synthesising a finding without a backing detector
  violates P1. A future enhancement may expose skip counts via a
  `--report-skips` flag; out of scope for v0.

The skip-and-continue policy keeps the `network-isolation` CI gate
(which runs `cntrdct scan` under `sudo unshare --net` with
`set -euo pipefail`) green even if a single file triggers
`StructuralInvariant`: the converter does not propagate the error
up to the scan driver.

### F3 — Location preservation

For every IR node `n` that carries a `Location`:

```
n.location.start_line == tree_sitter_node.start_position().row + 1
n.location.start_col  == tree_sitter_node.start_position().column + 1
n.location.end_line   == tree_sitter_node.end_position().row + 1
n.location.end_col    == tree_sitter_node.end_position().column + 1
n.location.start_byte == tree_sitter_node.start_byte()
n.location.end_byte   == tree_sitter_node.end_byte()
n.location.file       == IrFile.path
```

The line / column invariant is the contract `eval-v0.md` §F3 depends
on: the `evaluate(manifest, actual, corpus_dir)` matcher compares
`expected.line` against `actual.primary.start_line` for equality.
Drift here would silently break every recall figure in the audit
corpus.

The byte-offset invariant supports `clone-drift` F2b intra-fn
if-branch source-text equality (`normalize_block_source(consequence,
&file.source)` vs `normalize_block_source(alt_block, &file.source)`)
and `comment-code` Pattern B body-marker substring scan
(`body_text.contains(marker)`), both of which slice
`IrFile.source` by byte range rather than re-walking the tree.

R-1 ships a unit test per IR node kind that pins all six fields
against tree-sitter directly:

```rust
#[test]
fn ir_call_site_location_matches_tree_sitter() {
    let (tree, ir) = parse_to_ir("foo(x, y)\n", Language::Python);
    let ts_root = tree.root_node();
    let ts_call = first_descendant(ts_root, "call");
    let ir_call = ir.fns[0].body.statements.iter()
        .find_map(|s| match &s.kind { IrStmtKind::Call(c) => Some(c), _ => None })
        .unwrap();
    assert_eq!(ir_call.location.start_line,
               ts_call.start_position().row as u32 + 1);
    assert_eq!(ir_call.location.start_byte, ts_call.start_byte() as u32);
    assert_eq!(ir_call.location.end_byte,   ts_call.end_byte() as u32);
    // ... start_col, end_line, end_col, file
}
```

One such test per IR-bearing node kind (`IrCallSite`, `IrBlock`,
`IrFn`, `IrComment`, `IrIfStmt`, `IrWhileStmt`, `IrLoopStmt`,
`IrMatchStmt`, `IrWithStmt`, `IrStmtKind::Other`, `IrExpr::Other`,
`IrParam`, `IrDecorator`), exercised on both Rust and Python source.
For Python, the suite covers top-level functions, class methods,
nested calls, decorators, and docstrings; for Rust, top-level
functions, impl methods, nested if/match, and attribute-bearing
items. Lives in `tests/ir_location.rs`.

### F4 — Detector trait migration

The `Detector` trait changes its input type from `ParsedFile` to
`IrFile`. The trait surface itself does not parameterise on file
shape (we keep `&DetectContext<'a>` so the existing call shape in
detector tests survives), but the slice it holds switches:

Before (v0.5.x):

```rust
pub struct DetectContext<'a> {
    pub files: &'a [ParsedFile],
    pub stats: &'a CorpusStats,
    pub config: &'a DetectorConfig,
}
```

After (R-1, v0.6.0):

```rust
pub struct DetectContext<'a> {
    pub files: &'a [IrFile],
    pub stats: &'a CorpusStats,
    pub config: &'a DetectorConfig,
}
```

`ParsedFile` itself is deleted in R-1; no `ParsedFile::ir()`
compatibility shim is provided. Rationale:

- The only consumer of `Detector` outside the cntrdct crate is the
  cntrdct repository itself. There is no out-of-tree detector
  ecosystem to break.
- A shim adds maintenance burden during the transition window and
  invites detectors to remain on `ParsedFile` indefinitely. A clean
  cut means R-1 either lands or it does not; partial states are
  flagged by the compiler.
- The Q-15 baseline scaffolding retirement (R-1.e) is also a
  breaking change to the same release; co-locating both keeps the
  v0.6.0 changelog focused.

This is a `cntrdct` 0.5.x -> 0.6.0 bump. Conventional Commits prefix
for the R-1 commit: `feat(ir)!`. `CHANGELOG.md` records both the
IR migration and the baseline retirement under the `0.6.0` heading.

Cross-cutting spec sweep: this spec's `IrFile` references override
the `ParsedFile` mentions in `arg-swap-v0.md`, `clone-drift-v0.md`,
`comment-code-v0.md`, `unreachable-after-terminator-v0.md`,
`pr-miner-v0.md`, and `lsp-v0.md` (F1 input sections, scan_buffer
signature, anywhere else `ParsedFile` appears as a type). The R-1
commit applies a textual sweep across those six files in the same
PR as the IR implementation; reviewers verify the sweep by grepping
for `ParsedFile` under `docs/spec/` and asserting only the
historical commentary in this spec survives.

### F5 — Language-specific escape hatch

Detectors under `src/detectors/lang/` (the post-R-1 home for
`rust_config_interaction.rs` and the planned R-5 Python
`python_unreachable_except.rs`) implement the same `Detector` trait
but access tree-sitter directly via the IR's retained handles:

```rust
fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
    let mut findings = Vec::new();
    for ir_file in ctx.files {
        if ir_file.language != Language::Rust { continue; }
        if ir_file.parse_recovered { continue; }
        let raw_tree = ir_file.raw_tree();          // lazy reparse
        let root = raw_tree.root_node();
        // ... walk root directly using tree-sitter APIs.
        // `raw_tree` drops at end of this iteration, bounding peak
        // RSS by concurrent task count rather than corpus size.
    }
    Ok(findings)
}
```

The IR types are not opaque to lang detectors — they can also walk
`IrFile.fns` and so on — but the canonical pattern is to ignore IR
and reparse via `IrFile::raw_tree()` plus the `parse_recovered`
skip-gate.

Cross-cutting detectors must not call `raw_tree()`, with ONE
documented exception: `pr-miner`. Its concept is cross-cutting
(implicit programming-rule mining transfers across languages), but
its algorithm mines association rules over the SET of every
call-head last-segment in each function body — a full recursive AST
enumeration the structured IR does not losslessly preserve.
`IrCallSite.callee` flattens a method-chain receiver to name
segments and drops the receiver call itself (`a.b().c()` exposes
`c`, not `b`), and calls nested in `IrExpr::Other` shapes (`?`-try,
binary, index, …) are invisible to an IR-only walk. An empirical
probe over `benchmarks/wild-corpus` (270 files) found 65 % of
top-level functions yield a different call-head set under a pure-IR
walk (1750 missed heads), which would shift pr-miner's global
Apriori support / confidence and break the T1 pinning. pr-miner
therefore keeps the F5 escape hatch (a single per-file
`raw_tree()` reparse) rather than migrating; closing the gap would
require materialising call sites in receiver position and every
Other expression shape, i.e. reconstructing the full AST in IR.
This is a documentation-level discipline only; the type system does
not enforce it. A `clippy` lint or a `tests/raw_tree_discipline.rs`
check is a possible future hardening — out of scope for v0.

LSP non-regression. `cntrdct-lsp`'s `didChange` handler produces an
intermediate tree whose `root_node().has_error()` is true the
moment the user is mid-keystroke. Under F2's partial-parse rule
that yields `parse_recovered = true`, which every cross-cutting
detector skips. To preserve the v0.5.2 LSP UX (diagnostics survive
while typing rather than blinking out on every keystroke), the LSP
layer caches the most recent `IrFile` whose `parse_recovered` is
false and re-uses that cache when conversion of the in-progress
buffer would set `parse_recovered = true`. The cache key is the
document URI; eviction happens on `didClose` or successful
re-parse. This requirement lives on the LSP side, not the IR /
converter side — the IR's contract is "tell the caller whether
parsing recovered"; the policy of what to do with a recovered tree
is the caller's. The R-1 PR includes the LSP-side cache change
alongside the IR migration so the v0.6.0 release ships both.

### F6 — R-1 test plan

The R-0 commit ships only the spec (this file). The pinning tests
listed below land in R-1 alongside the implementation.

T1. Per-detector pinning tests. For each cross-cutting detector
    (`arg-swap`, `clone-drift`, `comment-code`,
    `unreachable-after-terminator`, `pr-miner`), serialise the
    findings the detector produces against the existing audit-corpus
    + wild-corpus fixtures via `cntrdct scan --json`, compare
    byte-for-byte against a checked-in golden snapshot captured from
    v0.5.2 before R-1.c begins. Any divergence fails the test. The
    snapshot lives at
    `tests/fixtures/ir-pinning/<detector>/{audit,wild}.json`.
    Snapshot capture is an explicit R-1 sub-step preceding R-1.c so
    the v0.5.2 behaviour is pinned before the rewrite begins.

T2. `IrConvertError` variant coverage. One fixture file per variant:
    a deliberately-language-mismatched call to `to_ir` (T2a), an
    empty-string source (T2b), and a tree whose top-level
    `function_item` has no `name` field that exercises the
    `StructuralInvariant` path under a fault-injection shim (T2c).
    Plus end-to-end tests that drive `cntrdct scan` against each
    fixture and assert (a) the scan exits 0, (b) the offending file
    is skipped (no findings emitted from it), and (c) the
    appropriate `tracing` log entry is captured per F2's
    production-runtime contract. Lives in `tests/ir_convert_error.rs`.

T3. `IrFile.parse_recovered` carry-through. A fixture file with a
    deliberate syntax error per language, asserting
    `parse_recovered == true` after `to_ir` and that every
    cross-cutting detector skips the file (zero findings) when
    `parse_recovered` is true. Plus an LSP integration smoke test
    that drives `didChange` with a deliberately broken buffer and
    asserts diagnostics from the prior successful parse remain
    visible. Lives in `tests/ir_recovery.rs`.

T4. IR golden fixtures. For a handful of canonical sources per
    language (one with a class / impl, one with nested calls, one
    with a deeply-nested if/match, and — once R-1.c'' step 2 lands the
    `For` / `Try` / `Assign` / `Let` variants — one exercising those
    nested statement bodies: `rust/let_for.rs` and
    `python/for_try_assign.py`) under
    `tests/fixtures/ir/<language>/`, serialise the converted
    `IrFile` to JSON and pin against a golden file. The test
    serialises a `SerializableIrFile` projection that omits `source`
    (reproducible from the fixture path) so the golden file stays
    diff-friendly. `IrFn.normalised_tokens` carries the function-item-
    rooted sequence per R2; each `IrBlock` serialises a
    `normalised_token_count` scalar. Catches converter regressions
    independent of detector behaviour.

T5. Location-equality unit tests per IR node kind, per F3. The
    suite spans both Rust and Python source and covers (per F3)
    top-level functions, class methods (Python), impl methods
    (Rust), nested calls, decorators / attributes, docstrings,
    nested if/match, and attribute-bearing items.

T6. Recall regression check. After R-1 lands but before tagging
    v0.6.0, run:

    ```sh
    cargo run --release -- calibrate --audit-recall benchmarks/audit-corpus
    ```

    and assert the resulting `overall_recall_upper_bound` is within
    the noise floor of the v0.5.2 baseline (>= 0.918 absolute, the
    same threshold codified in REBUILD-handoff.md's "エビデンス
    検証ルール"; corrected from the rounded 0.92 by the R-1.g
    re-measurement, which showed v0.5.2 itself produces 0.918 on the
    byte-identical audit corpus — see REBUILD.md §9 "Floor
    reconciliation"). T6 and the per-PR T1 pinning tests are
    complementary — T1 guards finding-set identity, T6 guards
    aggregate corpus recall. A T6 regression with T1 green
    indicates corpus / labelling drift, not converter drift.

T7. Wall-clock and peak-RSS measurement. Captured before R-1.e
    retires `tests/baselines.rs`, so the baseline harness is still
    available. The measurement runs `cntrdct scan` against
    `benchmarks/wild-corpus-python` (~600 files) at the v0.5.2 git
    SHA and again at the R-1 head SHA; reports both wall-clock and
    peak RSS deltas in the R-1 PR description. After R-1.e, a
    one-off `benches/wild_corpus_python.rs` criterion harness
    replaces `tests/baselines.rs` as the standing benchmark seam;
    R-1.c → T7 capture → R-1.e baseline retire is the binding
    sub-step ordering.

### F7 — Non-goals

- IR completeness. The converter only models the shapes the five
  cross-cutting detectors need. Unknown shapes become `Other` (with
  `node_kind` discriminator + `NodeRef` for raw-tree recovery).
- IR-to-source pretty-printing. The raw tree-sitter tree is the
  source of truth for round-trippable views; IR is consumption-only.
- IR persistence / caching across runs. Every `cntrdct scan` re-parses
  and re-converts. The LSP-side cache (F5) is a different concern:
  it caches IR within a session for UX, not across sessions.
- Variant additions to `Language` (`TypeScript`, `Go`, `Java`).
  Those land per R-2 / R-3 and may necessitate IR shape extensions,
  but the extensions are out of v0 scope.
- `Detector::detect` signature changes beyond the input type swap.
  Trait method names, return types, and error variants stay as in
  v0.5.x.
- Adjustments to `Citation` / `Evidence` / `Finding` types.
  `cntrdct-core`'s public surface outside `DetectContext` is
  unchanged.
- `#[non_exhaustive]` on `IrStmtKind` / `IrExpr` / `IrTerminator`
  for external consumers. The trait surface is internal and the
  cross-cutting detectors are part of the same crate; the v0
  enums are exhaustive against the v0 grammars. R-2 / R-3 revisit
  if a future variant addition is judged breaking for external
  consumers (currently there are none).

## Risks and open questions

R1. Tree-sitter `Tree` lifetime. The first R-1.c landing held one
`Arc<SyncTree>` per file alive for the entire scan; T7 on
`benchmarks/wild-corpus` (270 Rust files) measured a 5.4× peak-RSS
regression (71 → 380 MiB), exceeding the 25 % gate. Revised
mitigation: `IrFile` does not store the tree at all. `IrFile::
raw_tree()` is a method that reparses the source on every call and
returns a fresh `Arc<SyncTree>` that drops when the caller releases
it. Peak RSS now scales with concurrent detector tasks (rayon
worker count) rather than corpus size. Trade-off: each detector
that walks the raw tree pays one reparse per file; once the
cross-cutting detector IR migration completes (the deferred R-1.c
follow-up) the reparse cost goes away entirely. T7 confirms the
refactor cuts the regression to 2.3× peak RSS on wild-corpus Rust;
the residual is IR struct overhead and falls under a separate
R-1.c'' compaction item rather than re-litigating R1. The Sync
plumbing (`SyncTree` newtype + `unsafe impl Sync`) survives because
detector code can still hold `Arc<SyncTree>` across rayon tasks
while a single file is being processed.

R1 follow-up resolution (R-1.c'' path (a), 2026-06-03). The < 25 %
peak-RSS gate is **structurally unreachable** for the Rust wild-corpus
and has been retired-and-replaced by an absolute ceiling
(≤ 175 MiB), not a relative target. The cross-file detectors
(clone-drift, pr-miner) require the entire corpus's IR resident
simultaneously, so per-file retention cannot be dropped. A floor
study (`scan benchmarks/wild-corpus`, 270 files) measured peak RSS
~125 MiB even after emptying the two largest per-node contributors
(`IrFn.normalised_tokens` ≈ 33 MiB, per-node `Location.file` path
duplication ≈ 16 MiB) — +75 % over the 71.5 MiB v0.5.2 baseline, so
field compaction cannot reach the ≈ 89 MiB target. The safe,
T1-byte-identical compaction shipped: `Location.file` is a shared
`Arc<Path>` (one per-file allocation referenced by every node, vs a
per-node `to_path_buf()`), cutting Rust wild-corpus peak RSS
~169 → ~150 MiB. The 175 MiB ceiling has headroom over that and
still catches the regression class that matters (the original
eager-tree design measured 380 MiB). See REBUILD.md § 9 and R-1.c''.

R2. Normalised-token storage duplicates work the converter already
does to materialise `IrBlock.statements`. Eager pre-computation is the
conservative choice: the existing `clone-drift` pipeline normalises
every top-level function exactly once, and the IR layer does the same.

Placement (revised in R-1.c''): the full token sequence lives on
`IrFn.normalised_tokens`, rooted at the whole function item so the
signature prefix participates in `clone-drift`'s n-gram set (the
first R-1.c' landing rooted it at `IrFn.body`, dropping the prefix
and shifting pairwise Jaccard — see REBUILD.md R-1.c''). Per-block,
only a `normalised_token_count: usize` is stored (for F2b's
consequence size gate), not a per-block token vector: that keeps
block storage O(1) instead of O(tokens × nesting-depth). Cost is
O(total AST nodes) per file for the function walk plus the per-block
counts, the same complexity class as the v0.5.x scan. R-1 measures
wall-clock via T7; the cross-cutting detector IR migration (clone-drift
landed in R-1.c'') removes the per-detector `raw_tree()` reparse,
recovering the wall-clock the lazy-reparse design traded away.

R3. `IrTerminator::ConstantBranchUnreachable` exposes F4e Python
constant-condition classification in the IR layer. The classifier
itself is Python-specific (no Rust equivalent in v0; Rust constant-
condition is left to clippy). Two options were considered:

(a) Keep the classifier in IR (current spec): the Rust converter
    never emits `ConstantBranchUnreachable`, and the cross-cutting
    detector pipeline stays uniform across languages.
(b) Lift the classifier out of IR and into a Python-specific
    detector under `src/detectors/lang/`. The cross-cutting
    `unreachable-after-terminator` then loses its F4e coverage.

Spec selects (a). The F4e logic is a few lines and lives naturally
alongside the other terminator classifications. R-1 keeps the
existing tests for `constant-false-while` /
`constant-false-if` / `constant-true-if-else` green via the IR
pathway.

R4. `IrComment.target: Option<NodeRef>` points into `raw_tree` for
the "the item this comment documents" relationship. For Rust the
target is the immediately-following `function_item`; for Python the
target is the enclosing `function_definition` / `class_definition`
(the docstring lives inside, not before). Resolution is the
converter's responsibility; consumers (`comment-code`) only see
the resolved `NodeRef`. `None` means "no associated item" — both
languages emit standalone comments that we preserve verbatim but do
not pattern-match against function shapes. `IrFn.leading_doc` is
the pre-rendered convenience field for `comment-code`; the
detector reads `leading_doc` first and falls back to walking
`IrComment.target` only when it needs the structural delimiter /
kind information (Rust `///` vs `//!` distinction, Python
docstring quote style).

R5. The `IrFn.return_type_text: Option<String>` field is a raw
substring of the source, not a parsed type expression. The current
`comment-code` Rust Pattern A only ever does substring-contains
checks (`text.contains("Result")` / `text.contains("Option")`); the
IR layer preserves that shape rather than introducing a type-system
abstraction. The cost is that future detectors wanting structural
type analysis must re-parse from `raw_tree()`. Accepted.

R6. `IrStmtKind::Other` and `IrExpr::Other` are correctness-critical
escape hatches but provide no help for catch-all detection of new
language constructs. The `node_kind: &'static str` discriminator
mitigates the "fall back to raw_tree just to filter" cost: a
detector can match on `node_kind` strings to skip or short-circuit
without touching `raw_tree()`. A future cross-cutting detector that
needs (say) Python `try` statements will require an IR extension.
The process is: add the new variant to `IrStmtKind` / `IrExpr`,
update the per-language converters, ship as a `cntrdct` minor
version bump (`0.6.x` -> `0.7.0` if the variant addition is
breaking for existing match arms in non-tree-sitter detector code).
R-2 / TypeScript pilot is the first time this pressure-tests in
practice; the spec defers a formal addition / deprecation policy
until then.

R7. Test golden fixtures (T1, T4) are sensitive to JSON field
order and to non-Serde types. `tree_sitter::Tree` does not
implement `Serialize`; T4 serialises a `SerializableIrFile`
projection that strips `raw_tree` and `source` (both reproducible
from the fixture path) so the golden file stays diff-friendly.
The v0.5.x detector output is already pinned in
`tests/snapshots/` for some detectors; T1 follows the same
convention. The serialisation format for the surviving fields
follows Serde's struct-declaration order; the spec mandates
`IrFile`, `IrFn`, `IrBlock`, `IrStmt`, `IrStmtKind`, `IrCallSite`,
`IrPath`, `IrComment`, `IrDecorator`, `IrParam`, `IrTerminator`,
`IrLiteral`, `NormalisedToken`, `Location`, `DivergentKind`,
`BranchMergeKind`, `ConstantBranchKind`, `HoistedItemKind`,
`ParamKind`, `IrLabel`, `IrIfStmt`, `IrWhileStmt`, `IrLoopStmt`,
`IrMatchStmt`, `IrMatchArm`, `IrWithStmt` all derive `Serialize`
for this purpose. `NodeRef` derives `Serialize` via
`tree_sitter::Range`'s own `Serialize` impl (verified present in
the tree-sitter version cntrdct pins).

R8. LSP partial-parse UX. Without F5's cache requirement, the move
from v0.5.x's "skip files with `has_error()`" (silent per-detector
short-circuit) to v0.6.0's "skip files with `parse_recovered`"
(IR-layer flag) would have the same observable effect mid-
keystroke but would amplify the existing diagnostic-blink behaviour
because the LSP's `didChange` debounce (250ms) sits between the
last successful parse and the next. F5 mandates the LSP-side
cache; R-1 adds the cache implementation. R-8 risk: the cache key
is the document URI, not the buffer content, so a `didOpen` for a
broken buffer (no prior successful parse exists) still shows zero
diagnostics until the user fixes the syntax error. Accepted — it
matches v0.5.x behaviour for that specific case.

R9. `IrConvertError::StructuralInvariant` runtime contract. F2
defines the production behaviour (warn-log + per-file skip + no
synthetic SARIF finding). Risk: a converter bug that fires
`StructuralInvariant` on a real-world file is invisible to CI
until a user reports the warn-log entry. Mitigation: T2c's
fault-injection shim catches the failure shape; the standing
`benchmarks/wild-corpus-python` recall measurement (T6) catches
the symptom of widespread converter skips (recall drops sharply).
Residual exposure: a converter bug that fires on < 1 % of files
may stay invisible. R-2 / TypeScript pilot exercises a fresh
converter and is the next forcing function on the
`StructuralInvariant` runtime contract.

R10. Stale historical references in spec / docs.
`REBUILD-handoff.md` §"優先度低" lists 14 places where the retired
`ROADMAP.md` is still cross-referenced (`docs/spec/multilang-v0.md`,
`docs/spec/pr-miner-v0.md`, `docs/spec/recall-audit-v0.md`,
`docs/spec/citations-policy.md`, `docs/spec/cross-model-kappa-v0.md`,
`docs/spec/llm-calibration-v0.md`, `docs/spec/sota-baselines-v0.md`
— retired entirely by R-1 — `docs/spec/arg-swap-v0.md`,
`docs/spec/lsp-v0.md`, `book/src/{introduction,releases}.md`,
`docs/site/index.md`, three `benchmarks/*/README.md`,
`research/projects/PLAN.md`). R-1 PR resolves each per
REBUILD-handoff.md's three-way choice (preserve as history /
swap to REBUILD.md / delete). Not a converter risk; called out
here so the R-1 PR scope is unambiguous.

R11. Sub-step ordering within R-1. T7 (wall-clock + peak-RSS
measurement) MUST run before R-1.e (baseline retirement). R-1.f
(self-replication ledger introduction) reads the eval output, so
it depends on R-1.c (detector rewrite) completing. R-1.g (priors
recalibration) depends on R-1.c. R-1.h (Cargo.toml bump) depends
on all of the above. The binding order is:
R-1.a → R-1.b → R-1.c → T7 → R-1.d → R-1.e → R-1.f → R-1.g → R-1.h
→ R-1.i. T1 snapshot capture runs at v0.5.2 git SHA before R-1.a.
The R-1 PR description records the sub-step order explicitly.

## References

- `REBUILD.md` §1 G1 — architecture goal this spec realises.
- `REBUILD-handoff.md` — R-1 sub-step ordering, optional / required
  sweeps, and the エビデンス検証ルール recall threshold (>= 0.918,
  corrected from the rounded 0.92 by R-1.g) T6 inherits.
- `docs/spec/multilang-v0.md` — `ParserProvider` seam this spec
  extends. F3's `ParserProvider` trait gains a `to_ir` method per
  this spec's F2; the existing `language` and `ts_language` methods
  are unchanged.
- `docs/spec/citations-policy.md` — citation grounding policy.
  Unchanged by IR (the cross-cutting detectors keep their existing
  citation sets verbatim; new languages added under R-2 / R-3 follow
  the existing survey procedure).
- `docs/spec/eval-v0.md` §F3 — line-equality matching rule whose
  preservation is the load-bearing F3 invariant of this spec.
- `docs/spec/arg-swap-v0.md`, `clone-drift-v0.md`,
  `comment-code-v0.md`, `unreachable-after-terminator-v0.md`,
  `pr-miner-v0.md`, `lsp-v0.md` — the five cross-cutting detector
  specs (+ LSP) whose `ParsedFile` references are overridden by
  this spec's F4. R-1 commit applies the textual sweep.
- `src/parsers.rs`, `src/core.rs`, `src/lsp.rs` — implementation
  seams R-1 modifies. R-1.a creates `src/ir.rs`; R-1.b promotes
  `src/parsers.rs` to a directory and adds `to_ir` to each provider;
  the LSP cache change (F5) lands in the same commit.

## Approval

Draft, post-R-0-review revision. Once approved, R-1 implementation
proceeds per `REBUILD.md` §4 R-1 and `REBUILD-handoff.md` "R-1
詳細手順" with the R11 sub-step ordering above.
