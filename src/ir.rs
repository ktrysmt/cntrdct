//! Language-agnostic Intermediate Representation (IR) consumed by the
//! cross-cutting detectors.
//!
//! Spec: `docs/spec/ir-v0.md`.
//!
//! R-1.a scope: this file defines the IR node shape (§F1) plus the
//! [`IrConvertError`] variants (§F2) and the [`IrFile::resolve`]
//! contract. Conversion implementations live alongside the per-language
//! [`crate::parsers::ParserProvider`] entries; R-1.b adds them.
//!
//! Cross-cutting detectors (under `src/detectors/`) consume IR; they
//! must not touch [`IrFile::raw_tree`]. Language-specific detectors
//! (`src/detectors/lang/`) may walk the tree via the escape hatch
//! described in §F5.
//!
//! Lazy reparse (ir-v0.md R1 mitigation): [`IrFile`] does NOT store
//! the tree-sitter `Tree` across the scan. Calling
//! [`IrFile::raw_tree`] reparses the source on demand and returns a
//! fresh [`SyncTree`] that drops when the caller releases the
//! returned `Arc`. This trades reparse CPU per detector access for
//! memory (no all-files tree retention).
//!
//! Serialization. Every IR node except [`IrFile`] derives
//! [`serde::Serialize`] so the §F6 T4 golden fixtures can pin
//! converter output. The test suite serializes a
//! `SerializableIrFile` projection that strips `source` (reproducible
//! from the fixture path). [`NodeRef`] supplies a
//! hand-written `Serialize` impl because `tree_sitter::Range` does not
//! itself implement `Serialize` in the pinned tree-sitter version.

#![deny(missing_docs)]

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

pub use crate::core::Language;

// ---------- Location ----------

/// Source range carried by every IR node.
///
/// All six positional fields are pinned against `tree_sitter::Node`
/// per ir-v0.md §F3. Line / column are 1-based (tree-sitter's
/// 0-based `Point` plus 1); byte offsets are zero-based into
/// [`IrFile::source`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// Path of the source file; equals [`IrFile::path`].
    pub file: PathBuf,
    /// 1-based line of the first character of the span.
    pub start_line: u32,
    /// 1-based column of the first character of the span.
    pub start_col: u32,
    /// 1-based line of the character immediately after the span.
    pub end_line: u32,
    /// 1-based column of the character immediately after the span.
    pub end_col: u32,
    /// Byte offset of the first character of the span into
    /// [`IrFile::source`].
    pub start_byte: u32,
    /// Byte offset immediately after the span into [`IrFile::source`].
    pub end_byte: u32,
}

// ---------- NodeRef ----------

/// Opaque reference into the tree returned by [`IrFile::raw_tree`].
///
/// Only meaningful when paired with the [`IrFile`] it was created
/// from. Crossing the two — using a `NodeRef` from one [`IrFile`]
/// against another — is a programmer error and returns `None`
/// from [`IrFile::resolve_with`].
#[derive(Debug, Clone)]
pub struct NodeRef {
    /// Raw tree-sitter range used as the lookup key.
    pub range: tree_sitter::Range,
}

impl Serialize for NodeRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("NodeRef", 6)?;
        s.serialize_field("start_byte", &self.range.start_byte)?;
        s.serialize_field("end_byte", &self.range.end_byte)?;
        s.serialize_field("start_row", &self.range.start_point.row)?;
        s.serialize_field("start_col", &self.range.start_point.column)?;
        s.serialize_field("end_row", &self.range.end_point.row)?;
        s.serialize_field("end_col", &self.range.end_point.column)?;
        s.end()
    }
}

// ---------- SyncTree wrapper ----------

/// Thread-safe wrapper around [`tree_sitter::Tree`] so [`IrFile`] can
/// be shared across rayon work-stealing tasks without breaking the
/// existing per-file `par_iter()` pattern in detectors.
///
/// `tree_sitter::Tree` (0.22) implements `Send` but not `Sync`; the
/// underlying `TSTree` is a refcounted C struct whose immutable
/// (`&self`) methods are safe to call concurrently — only `edit(&mut
/// self, …)` mutates and IR never calls it. `unsafe impl Sync` is the
/// minimum addition that lets `Arc<SyncTree>` be `Sync` so
/// `&[IrFile]: IntoParallelIterator` continues to compile.
///
/// The wrapper is `#[repr(transparent)]` and derefs to
/// `tree_sitter::Tree` so callers continue to write
/// `raw_tree.root_node()` unchanged.
#[derive(Debug)]
#[repr(transparent)]
pub struct SyncTree(tree_sitter::Tree);

impl SyncTree {
    /// Wrap an owned [`tree_sitter::Tree`].
    pub fn new(tree: tree_sitter::Tree) -> Self {
        Self(tree)
    }

    /// Borrow the underlying tree.
    pub fn inner(&self) -> &tree_sitter::Tree {
        &self.0
    }
}

impl std::ops::Deref for SyncTree {
    type Target = tree_sitter::Tree;
    fn deref(&self) -> &tree_sitter::Tree {
        &self.0
    }
}

// Safety: the IR layer (and every consumer) only ever calls `&self`
// methods on the wrapped `Tree`. `tree_sitter::Tree::edit` (the sole
// `&mut self` mutator) is never invoked. Immutable C-level reads
// against the underlying `TSTree` are documented as thread-safe.
unsafe impl Sync for SyncTree {}

// ---------- IrFile ----------

/// One source file converted into IR.
///
/// The tree-sitter `Tree` is NOT stored as a field. Call
/// [`IrFile::raw_tree`] for a fresh reparse — the returned `Arc`
/// drops when the caller releases it, bounding peak memory at the
/// number of concurrent detector tasks rather than the corpus size
/// (ir-v0.md R1 mitigation). `Serialize` is intentionally not
/// derived because `tree_sitter::Tree` is not serializable; T4
/// fixtures project to a stripped struct (see module docs).
#[derive(Debug, Clone)]
pub struct IrFile {
    /// Filesystem path of the source file.
    pub path: PathBuf,
    /// Language of the source file.
    pub language: Language,
    /// File contents as UTF-8, shared via `Arc` so IR nodes can refer
    /// to substrings without cloning.
    pub source: Arc<str>,
    /// Top-level functions in source order.
    pub fns: Vec<IrFn>,
    /// Free-standing comments not bound to a function.
    pub top_level_comments: Vec<IrComment>,
    /// True when tree-sitter's `root_node().has_error()` was true.
    /// Cross-cutting detectors gate on this to preserve the v0.5.x
    /// "skip files with parse errors" behaviour.
    pub parse_recovered: bool,
}

impl IrFile {
    /// Parse [`Self::source`] with the per-language tree-sitter
    /// grammar and return a fresh [`SyncTree`] wrapped in `Arc`.
    ///
    /// Each call produces an independent tree. Callers bind it to a
    /// local variable so the tree drops at end of scope; this is the
    /// R1 mitigation (no all-files tree retention across the scan).
    /// Cross-cutting detectors that still walk the raw tree pay one
    /// reparse per file per detector; once their IR migration lands
    /// the reparse goes away.
    ///
    /// Reparse cannot fail: the same source already parsed once at
    /// `to_ir` time. A panic here would indicate a tree-sitter
    /// version mismatch or an external `source` mutation, both
    /// outside the IR contract.
    pub fn raw_tree(&self) -> Arc<SyncTree> {
        let provider = crate::parsers::parser_for(self.language);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&provider.ts_language())
            .expect("tree-sitter language constructor is infallible");
        let tree = parser
            .parse(self.source.as_ref(), None)
            .expect("source already parsed successfully once at to_ir time");
        Arc::new(SyncTree::new(tree))
    }

    /// Resolve a [`NodeRef`] against a freshly-parsed tree and pass
    /// the matching node to `f`. The tree is dropped when `f`
    /// returns.
    ///
    /// Returns `None` when the ref was produced against a different
    /// [`IrFile`] (the caller misused the API).
    pub fn resolve_with<R>(
        &self,
        node_ref: &NodeRef,
        f: impl FnOnce(tree_sitter::Node<'_>) -> R,
    ) -> Option<R> {
        let raw = self.raw_tree();
        find_node_with_range(raw.root_node(), node_ref.range).map(f)
    }
}

fn find_node_with_range(
    node: tree_sitter::Node<'_>,
    target: tree_sitter::Range,
) -> Option<tree_sitter::Node<'_>> {
    if node.range() == target {
        return Some(node);
    }
    let node_range = node.range();
    if target.start_byte < node_range.start_byte || target.end_byte > node_range.end_byte {
        return None;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_node_with_range(child, target) {
            return Some(found);
        }
    }
    None
}

// ---------- IrFn / IrParam ----------

/// A function (or method) definition.
#[derive(Debug, Clone, Serialize)]
pub struct IrFn {
    /// Function name.
    pub name: String,
    /// Parameters in source order.
    pub params: Vec<IrParam>,
    /// Function body.
    pub body: IrBlock,
    /// Raw return-type text (Rust `-> T` suffix or Python
    /// `function_definition.return_type`). `None` when absent.
    pub return_type_text: Option<String>,
    /// Decorators (Python) or outer attributes (Rust) in source order.
    pub decorators: Vec<IrDecorator>,
    /// True when this function is a class method (Python) or impl
    /// item (Rust). `arg-swap` uses this to drop the leading
    /// `self` / `cls` parameter before arity checks.
    pub is_method: bool,
    /// Leading doc text in canonical, prefix-stripped form. `None`
    /// when no leading doc is present.
    pub leading_doc: Option<String>,
    /// Normalised token sequence rooted at the whole function item
    /// (Rust `function_item`, Python `function_definition`), used by
    /// `clone-drift`'s function-level clustering. Rooting at the
    /// function item — rather than the body block — preserves the
    /// v0.5.x `walk_normalize_*(function_item)` sequence byte-for-byte
    /// so the signature prefix participates in the n-gram set. Leaf
    /// tokens for identifiers and literals are folded to the
    /// placeholder kinds in `NormalisedToken`; comment nodes are
    /// excluded. Populated once per function (ir-v0.md R2).
    pub normalised_tokens: Vec<NormalisedToken>,
    /// Source location of the function definition.
    pub location: Location,
}

/// A function parameter.
#[derive(Debug, Clone, Serialize)]
pub struct IrParam {
    /// Parameter name (as spelled in source).
    pub name: String,
    /// Classification used by `arg-swap` for receiver dropping and
    /// unsupported-shape rejection.
    pub kind: ParamKind,
    /// Source location of the parameter.
    pub location: Location,
}

/// Parameter classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ParamKind {
    /// Plain positional parameter the detector can reason about.
    Plain,
    /// Implicit receiver (Python `self` / `cls` first parameter).
    Receiver,
    /// Parameter shape the cross-cutting layer cannot model
    /// (`*args`, `**kwargs`, `/`, `*` separators, Rust `self`
    /// patterns the converter does not unwrap). `arg-swap` rejects
    /// the entire function definition when any param is
    /// `Unsupported`.
    Unsupported,
}

// ---------- IrBlock ----------

/// A block of statements with pre-computed terminator + normalised
/// token sequence.
#[derive(Debug, Clone, Serialize)]
pub struct IrBlock {
    /// Statements in source order, excluding comment nodes.
    pub statements: Vec<IrStmt>,
    /// Terminator classification for the block as a whole. `Some`
    /// iff every reachable path through the block ends in a
    /// divergent expression.
    pub terminator: Option<IrTerminator>,
    /// Count of normalised tokens this block would produce when walked
    /// in isolation (block-rooted `walk_normalize_*`).
    ///
    /// `clone-drift`'s F2b intra-fn `if`-same-then-else gate reads the
    /// consequence block's count for its size threshold and finding
    /// message. Only the count is stored — not the token vector — so
    /// the per-block memory cost stays O(1) rather than the
    /// O(tokens × nesting-depth) a per-block vector would incur
    /// (ir-v0.md R2). The function-level token sequence lives on
    /// [`IrFn::normalised_tokens`].
    pub normalised_token_count: usize,
    /// Source location of the block.
    pub location: Location,
}

// ---------- IrStmt / IrStmtKind ----------

/// A statement with its preceding attribute / decorator chain.
#[derive(Debug, Clone, Serialize)]
pub struct IrStmt {
    /// Statement kind.
    pub kind: IrStmtKind,
    /// Attributes (Rust `#[cfg(...)]` immediately preceding the
    /// statement in the same block) or decorators that bind to
    /// this statement. Python attaches decorators to function /
    /// class definitions, not arbitrary statements; for Python
    /// per-statement contexts this is the empty vector.
    pub attributes: Vec<IrDecorator>,
    /// Source location of the statement.
    pub location: Location,
}

/// Statement kinds modelled by IR.
#[derive(Debug, Clone, Serialize)]
pub enum IrStmtKind {
    /// A function call sitting as a statement.
    Call(IrCallSite),
    /// Rust `let <pat> = <value>;`. `value` is the initialiser
    /// expression (`None` for an uninitialised `let x;`). The
    /// converter materialises the RHS so a cross-cutting detector can
    /// reach call sites and nested terminators hiding inside it
    /// (e.g. `let x = { return ...; };`) without dropping to
    /// `raw_tree()`. The binding pattern is not modelled — no
    /// cross-cutting detector reasons about the LHS name.
    Let {
        /// Initialiser expression, if present.
        value: Option<IrExpr>,
    },
    /// Python `<lhs> = <value>` (plain `assignment`). `value` is the
    /// RHS expression (`None` for an annotation-only `x: int` with no
    /// initialiser). Like [`IrStmtKind::Let`], the RHS is materialised
    /// so calls inside an assignment-wrapped statement
    /// (`_ = copy(src, dst)`) are visible to an IR-only walk.
    /// Augmented assignments (`x += 1`) keep the `Other` shape.
    Assign {
        /// RHS expression, if present.
        value: Option<IrExpr>,
    },
    /// `return <expr>` (or valueless `return`).
    Return(Option<IrExpr>),
    /// `raise <expr>` (Python) including the bare re-raise form.
    Raise(Option<IrExpr>),
    /// `break` with an optional label (Rust).
    Break(Option<IrLabel>),
    /// `continue` with an optional label (Rust).
    Continue(Option<IrLabel>),
    /// `assert <cond>` (Python) or `assert!(<cond>)` (Rust).
    Assert(IrExpr),
    /// Macro / call shapes whose semantics are divergent
    /// (`panic!()`, `sys.exit(...)`, etc.).
    DivergentCall {
        /// Canonical terminator kind.
        kind: DivergentKind,
        /// Arguments at the call site.
        args: Vec<IrExpr>,
    },
    /// `if` statement.
    If(IrIfStmt),
    /// `while` loop.
    While(IrWhileStmt),
    /// Rust `loop { ... }`.
    Loop(IrLoopStmt),
    /// `for <pat> in <iterable>` loop (Rust `for_expression`, Python
    /// `for_statement`). The iterable expression and the loop body are
    /// both materialised so call sites in either position are visible
    /// to an IR-only walk. The loop variable pattern is not modelled.
    For(IrForStmt),
    /// `match` (Rust) or pattern-matched statement form.
    Match(IrMatchStmt),
    /// Python `with <ctx> as <name>: <body>`.
    With(IrWithStmt),
    /// Python `try: <body> except ...: <handler> [else] [finally]`.
    /// Every sub-block is materialised so calls under any clause are
    /// reachable from an IR walk. The exception-type / binding shapes
    /// in the `except` clauses are not modelled.
    Try(IrTryStmt),
    /// A nested item declaration the compiler hoists out of
    /// statement order. Lets `unreachable-after-terminator` skip
    /// hoisted items without recovering the raw node.
    HoistedItem {
        /// Hoisted-item classification.
        kind: HoistedItemKind,
        /// Reference back into the raw tree-sitter tree.
        node_ref: NodeRef,
    },
    /// Any statement shape the converter does not model.
    Other {
        /// Tree-sitter node kind string for fast filtering.
        node_kind: &'static str,
        /// Reference back into the raw tree-sitter tree.
        node_ref: NodeRef,
    },
}

// ---------- Control-flow statement payloads ----------

/// `if` statement (or expression in statement position).
#[derive(Debug, Clone, Serialize)]
pub struct IrIfStmt {
    /// Condition expression.
    pub condition: IrExpr,
    /// Consequence block.
    pub consequence: IrBlock,
    /// Alternative block (else / else-if chain unwrapped by the
    /// converter), or `None` when absent.
    pub alternative: Option<IrBlock>,
    /// Pre-computed branch-merge terminator. `Some(BranchMerge { ... })`
    /// when every branch ends in a divergent expression.
    pub terminator: Option<IrTerminator>,
    /// Source location of the `if`.
    pub location: Location,
}

/// `while` loop.
#[derive(Debug, Clone, Serialize)]
pub struct IrWhileStmt {
    /// Loop condition.
    pub condition: IrExpr,
    /// Loop body.
    pub body: IrBlock,
    /// Source location of the loop.
    pub location: Location,
}

/// Rust `loop { ... }`.
#[derive(Debug, Clone, Serialize)]
pub struct IrLoopStmt {
    /// Optional `'label` (Rust labelled loop).
    pub label: Option<IrLabel>,
    /// Loop body.
    pub body: IrBlock,
    /// True when the body contains a `break` whose target is this
    /// loop. `unreachable-after-terminator` uses this to classify
    /// `loop { ... }` as divergent when no break targets it.
    pub has_break_to_self: bool,
    /// Source location of the loop.
    pub location: Location,
}

/// `match` statement.
#[derive(Debug, Clone, Serialize)]
pub struct IrMatchStmt {
    /// Scrutinee expression.
    pub scrutinee: IrExpr,
    /// Match arms in source order.
    pub arms: Vec<IrMatchArm>,
    /// Pre-computed branch-merge terminator. `Some(BranchMerge { ... })`
    /// when every arm body ends in a divergent expression.
    pub terminator: Option<IrTerminator>,
    /// Source location of the match.
    pub location: Location,
}

/// One arm of a `match`.
#[derive(Debug, Clone, Serialize)]
pub struct IrMatchArm {
    /// Arm body expression (typically an [`IrExprKind::Block`]).
    pub body: IrExpr,
    /// Source location of the arm.
    pub location: Location,
}

/// Python `with` statement.
#[derive(Debug, Clone, Serialize)]
pub struct IrWithStmt {
    /// Context-manager expressions in source order.
    pub context_managers: Vec<IrExpr>,
    /// Body block.
    pub body: IrBlock,
    /// Source location of the `with`.
    pub location: Location,
}

/// `for` loop (Rust `for_expression`, Python `for_statement`).
#[derive(Debug, Clone, Serialize)]
pub struct IrForStmt {
    /// Iterable expression (Rust `for_expression.value`, Python
    /// `for_statement.right`).
    pub iterable: IrExpr,
    /// Loop body.
    pub body: IrBlock,
    /// Source location of the `for`.
    pub location: Location,
}

/// Python `try` statement.
#[derive(Debug, Clone, Serialize)]
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
    /// Source location of the `try`.
    pub location: Location,
}

// ---------- IrCallSite / IrPath ----------

/// A resolved function or method call.
#[derive(Debug, Clone, Serialize)]
pub struct IrCallSite {
    /// Resolved callee path (receiver chain + segments).
    pub callee: IrPath,
    /// Argument expressions in source order.
    pub args: Vec<IrExpr>,
    /// Source location of the call site.
    pub location: Location,
}

/// A dotted / scoped path with optional receiver chain.
///
/// Python `self.method` populates `receiver = ["self"]` and
/// `segments = ["method"]`. Python `obj.attr.method` populates
/// `receiver = ["obj", "attr"]` and `segments = ["method"]`. Rust
/// `foo::bar::baz` populates an empty receiver and
/// `segments = ["foo", "bar", "baz"]`.
#[derive(Debug, Clone, Serialize)]
pub struct IrPath {
    /// Receiver chain (zero or more segments before the final name).
    pub receiver: Vec<String>,
    /// Path segments in source order.
    pub segments: Vec<String>,
    /// Raw source text of the callee, preserving the original
    /// spelling (`foo::bar::baz` vs `foo.bar.baz`).
    pub raw: String,
}

// ---------- IrExpr ----------

/// An expression with its source location.
///
/// Carries [`Location`] on every expression (not just `Other`) so
/// `unreachable-after-terminator` can report F4d-ii / F4d-iii / F4d-iv
/// (divergent call argument / divergent return-or-break value /
/// divergent `if` condition) and F4e (Python constant condition)
/// finding endpoints at the exact source span the v0.5.x raw-tree walk
/// used, without dropping to `raw_tree()` (R-1.c'' Path b). The
/// `location` of a [`IrExprKind::Call`] / `Block` / `If` / `Match` /
/// `Loop` expression equals the location stored on the boxed inner node
/// (same tree-sitter node); the duplication keeps every `IrExpr`
/// uniformly self-describing.
#[derive(Debug, Clone, Serialize)]
pub struct IrExpr {
    /// Expression kind.
    pub kind: IrExprKind,
    /// Source location of the expression.
    pub location: Location,
}

/// Expression kinds modelled by IR.
#[derive(Debug, Clone, Serialize)]
pub enum IrExprKind {
    /// Bare identifier.
    Ident(String),
    /// Dotted / scoped path.
    Path(IrPath),
    /// Literal value.
    Literal(IrLiteral),
    /// Function or method call.
    Call(Box<IrCallSite>),
    /// `return <expr>`; `None` is the valueless form.
    Return(Option<Box<IrExpr>>),
    /// `raise <expr>`; `None` is the Python bare re-raise form.
    Raise(Option<Box<IrExpr>>),
    /// `break` with an optional label.
    Break(Option<IrLabel>),
    /// `continue` with an optional label.
    Continue(Option<IrLabel>),
    /// Block expression.
    Block(Box<IrBlock>),
    /// `if` expression.
    If(Box<IrIfStmt>),
    /// `match` expression.
    Match(Box<IrMatchStmt>),
    /// `loop` expression.
    Loop(Box<IrLoopStmt>),
    /// Divergent macro / function call as expression.
    DivergentCall {
        /// Canonical terminator kind.
        kind: DivergentKind,
        /// Arguments at the call site.
        args: Vec<IrExpr>,
    },
    /// Any expression shape the converter does not model.
    Other {
        /// Tree-sitter node kind string for fast filtering.
        node_kind: &'static str,
        /// Reference back into the raw tree-sitter tree.
        node_ref: NodeRef,
    },
}

// ---------- IrLiteral ----------

/// Literal value kinds.
#[derive(Debug, Clone, Serialize)]
pub enum IrLiteral {
    /// Boolean literal.
    Bool(bool),
    /// Decimal integer when the literal parses cleanly via
    /// `i128::from_str_radix(_, 10)`. Hex / octal / binary literals
    /// stay `None` per F4e.
    Int(Option<i128>),
    /// Float literal (value not preserved).
    Float,
    /// String literal.
    String {
        /// `true` when the literal carries no content between its
        /// delimiters (used by F4e Python constant-condition
        /// classification).
        is_empty: bool,
    },
    /// Character literal.
    Char,
    /// Python `None`.
    None,
}

// ---------- NormalisedToken ----------

/// Token kinds emitted by the `clone-drift` normaliser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum NormalisedToken {
    /// Structural AST node kind (e.g. `"block"`, `"if_expression"`).
    Kind(&'static str),
    /// Identifier placeholder.
    Ident,
    /// Integer literal placeholder.
    LitInt,
    /// Float literal placeholder.
    LitFloat,
    /// String literal placeholder.
    LitStr,
    /// Character literal placeholder.
    LitChar,
    /// Boolean literal placeholder.
    LitBool,
}

// ---------- IrComment ----------

/// A comment retained for `comment-code` and related detectors.
#[derive(Debug, Clone, Serialize)]
pub struct IrComment {
    /// Comment kind (language + delimiter family).
    pub kind: IrCommentKind,
    /// Rendered text with the comment delimiter stripped. Multi-line
    /// comments join with `\n`.
    pub text: String,
    /// Reference to the documented item, if any (resolved by the
    /// converter — see ir-v0.md §R4).
    pub target: Option<NodeRef>,
    /// Source location of the comment.
    pub location: Location,
}

/// Comment delimiter / language family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IrCommentKind {
    /// Rust `/// ...` doc-comment line.
    RustDocLine,
    /// Rust `/** ... */` doc-comment block.
    RustDocBlock,
    /// Rust `// ...` line comment.
    RustLine,
    /// Rust `/* ... */` block comment.
    RustBlock,
    /// Python `# ...` comment.
    PythonComment,
    /// Python `"""..."""` docstring (or `'''...'''`).
    PythonDocstring,
}

// ---------- IrDecorator ----------

/// A decorator (Python) or attribute (Rust).
#[derive(Debug, Clone, Serialize)]
pub struct IrDecorator {
    /// Raw decorator / attribute text including delimiters.
    pub raw: String,
    /// Dotted name path. Rust `#[deprecated]` produces
    /// `["deprecated"]`; Python `@warnings.deprecated` produces
    /// `["warnings", "deprecated"]`. The last segment is what
    /// `comment-code` Pattern C and py-deprecated match against.
    pub name_path: Vec<String>,
    /// Source location of the decorator.
    pub location: Location,
}

// ---------- IrTerminator ----------

/// Pre-computed terminator classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IrTerminator {
    /// Control returns from the enclosing function.
    Return,
    /// Control unwinds via a raised exception.
    Raise,
    /// Control breaks out of the enclosing loop.
    Break,
    /// Control continues at the loop head.
    Continue,
    /// Divergent macro / function call.
    DivergentCall {
        /// Canonical divergent-call kind.
        kind: DivergentKind,
    },
    /// `assert false` / `assert!(false)` / `assert False`.
    AssertFalse,
    /// `loop { ... }` with no break-target.
    LoopNoBreak,
    /// `if` / `match` whose every branch ends in a divergent
    /// expression.
    BranchMerge {
        /// Which branching form caused the merge.
        kind: BranchMergeKind,
    },
    /// F4e Python `while False:` (body's first statement is
    /// unreachable).
    ConstantFalseWhile,
    /// F4e Python `if False:` consequence or `if True: ... else:`
    /// alternative.
    ConstantBranchUnreachable {
        /// Which constant-branch shape produced the terminator.
        kind: ConstantBranchKind,
    },
}

// ---------- Classifier enums ----------

/// Canonical names for divergent macro / function calls.
///
/// Replaces a stringly-typed `kind` so `match` arms get
/// exhaustiveness checking and adding a new variant is a
/// compile-time event at every consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    /// Python builtin `exit(...)`.
    ExitBuiltin,
    /// Python builtin `quit(...)`.
    QuitBuiltin,
}

/// Branch-merge subkind for [`IrTerminator::BranchMerge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BranchMergeKind {
    /// Every branch of an `if` chain diverges.
    IfBranchesDiverge,
    /// Every arm of a `match` diverges.
    MatchArmsDiverge,
}

/// Constant-branch subkind for
/// [`IrTerminator::ConstantBranchUnreachable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConstantBranchKind {
    /// `if False:` consequence is unreachable.
    ConstantFalseIf,
    /// `if True: ... else:` alternative is unreachable.
    ConstantTrueIfElse,
}

/// Hoisted-item subkind for [`IrStmtKind::HoistedItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HoistedItemKind {
    /// Rust `function_item` / `function_signature_item` nested
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

// ---------- IrLabel ----------

/// Label on a `break` / `continue` / labelled loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum IrLabel {
    /// Named label (Rust `'foo`).
    Named(String),
    /// No explicit label — applies to the innermost enclosing loop.
    Unlabelled,
}

// ---------- SerializableIrFile (test-only projection) ----------

/// `Serialize`-friendly projection of [`IrFile`] used by ir-v0.md §F6
/// T4 golden fixtures. Strips [`IrFile::source`] (reproducible from
/// the fixture path) so the JSON wire shape stays diff-friendly.
/// Field declaration order matches the spec §R7 list.
#[derive(Debug, Clone, Serialize)]
pub struct SerializableIrFile {
    /// Filesystem path of the source file (mirrors [`IrFile::path`]).
    pub path: PathBuf,
    /// Language of the source file (mirrors [`IrFile::language`]).
    pub language: Language,
    /// Top-level functions in source order.
    pub fns: Vec<IrFn>,
    /// Free-standing comments not bound to a function.
    pub top_level_comments: Vec<IrComment>,
    /// Tree-sitter recovery flag (mirrors [`IrFile::parse_recovered`]).
    pub parse_recovered: bool,
}

impl From<&IrFile> for SerializableIrFile {
    fn from(ir: &IrFile) -> Self {
        SerializableIrFile {
            path: ir.path.clone(),
            language: ir.language,
            fns: ir.fns.clone(),
            top_level_comments: ir.top_level_comments.clone(),
            parse_recovered: ir.parse_recovered,
        }
    }
}

// ---------- IrConvertError ----------

/// Errors a [`crate::parsers::ParserProvider::to_ir`] implementation
/// may return.
///
/// Production-runtime contract per ir-v0.md §F2:
///
/// - `EmptySource`: caller silently skips the file (no log).
/// - `LanguageMismatch`: caller logs at `tracing::error!` and skips.
/// - `StructuralInvariant`: caller logs at `tracing::warn!` and
///   skips; no synthetic SARIF finding is emitted (would violate
///   P1).
#[derive(Debug, Error)]
pub enum IrConvertError {
    /// The parser failed to set the requested tree-sitter language.
    /// Indicates a build / dependency bug, not a source-file
    /// problem.
    #[error("language mismatch: expected {expected:?}, got {actual}")]
    LanguageMismatch {
        /// Language the file walker selected.
        expected: Language,
        /// Tree-sitter language name actually loaded.
        actual: String,
    },
    /// Source is empty or whitespace-only.
    #[error("empty source")]
    EmptySource,
    /// Internal converter invariant failure.
    #[error("structural invariant on {kind}: {message}")]
    StructuralInvariant {
        /// Offending tree-sitter node kind for diagnosis.
        kind: &'static str,
        /// Detail message for the warn-log entry.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> Location {
        Location {
            file: PathBuf::from("a.rs"),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
            start_byte: 0,
            end_byte: 0,
        }
    }

    #[test]
    fn location_round_trips_via_serde() {
        let original = Location {
            file: PathBuf::from("src/foo.rs"),
            start_line: 3,
            start_col: 5,
            end_line: 4,
            end_col: 1,
            start_byte: 12,
            end_byte: 27,
        };
        let json = serde_json::to_string(&original).expect("serializes");
        assert!(json.contains("\"start_byte\":12"));
        assert!(json.contains("\"end_byte\":27"));
    }

    #[test]
    fn node_ref_serializes_with_stable_field_set() {
        let range = tree_sitter::Range {
            start_byte: 4,
            end_byte: 11,
            start_point: tree_sitter::Point { row: 0, column: 4 },
            end_point: tree_sitter::Point { row: 0, column: 11 },
        };
        let json = serde_json::to_string(&NodeRef { range }).expect("serializes");
        // Field set is hand-written; pin the wire shape so T4 goldens
        // stay diff-friendly across tree-sitter version bumps.
        assert_eq!(
            json,
            "{\"start_byte\":4,\"end_byte\":11,\"start_row\":0,\"start_col\":4,\"end_row\":0,\"end_col\":11}"
        );
    }

    #[test]
    fn ir_file_resolve_walks_raw_tree() {
        // Parse a tiny Rust source so we have a real tree to resolve
        // against. Uses the existing parsers seam.
        let source: Arc<str> = Arc::from("fn main() { foo(); }\n");
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::language())
            .expect("set rust language");
        let tree = parser
            .parse(source.as_ref(), None)
            .expect("parse rust source");
        let root = tree.root_node();
        let function = root.child(0).expect("function_item");
        let target_range = function.range();

        let ir_file = IrFile {
            path: PathBuf::from("demo.rs"),
            language: Language::Rust,
            source: Arc::clone(&source),
            fns: Vec::new(),
            top_level_comments: Vec::new(),
            parse_recovered: false,
        };
        let resolved_range = ir_file
            .resolve_with(
                &NodeRef {
                    range: target_range,
                },
                |n| n.range(),
            )
            .expect("range present in tree");
        assert_eq!(resolved_range, target_range);
    }

    #[test]
    fn ir_file_resolve_returns_none_for_foreign_range() {
        let source: Arc<str> = Arc::from("fn a() {}\n");
        let ir_file = IrFile {
            path: PathBuf::from("demo.rs"),
            language: Language::Rust,
            source,
            fns: Vec::new(),
            top_level_comments: Vec::new(),
            parse_recovered: false,
        };
        // A range past the end of the file does not match any node.
        let bogus = NodeRef {
            range: tree_sitter::Range {
                start_byte: 9_999,
                end_byte: 10_000,
                start_point: tree_sitter::Point {
                    row: 999,
                    column: 0,
                },
                end_point: tree_sitter::Point {
                    row: 999,
                    column: 1,
                },
            },
        };
        assert!(ir_file.resolve_with(&bogus, |n| n.range()).is_none());
    }

    #[test]
    fn ir_stmt_call_serializes_with_kind_and_attributes() {
        // Sanity-check that the §R7 declaration order produces the
        // documented field set (kind + attributes + location).
        let stmt = IrStmt {
            kind: IrStmtKind::Call(IrCallSite {
                callee: IrPath {
                    receiver: vec![],
                    segments: vec!["foo".to_string()],
                    raw: "foo".to_string(),
                },
                args: vec![],
                location: loc(),
            }),
            attributes: vec![],
            location: loc(),
        };
        let json = serde_json::to_string(&stmt).expect("serializes");
        assert!(json.contains("\"kind\":{\"Call\":"));
        assert!(json.contains("\"attributes\":[]"));
    }

    #[test]
    fn ir_convert_error_messages_carry_kind_and_message() {
        let err = IrConvertError::StructuralInvariant {
            kind: "function_item",
            message: "missing name field".to_string(),
        };
        let rendered = format!("{err}");
        assert!(rendered.contains("function_item"));
        assert!(rendered.contains("missing name field"));
    }

    /// Compile-time `Send` check used by `tests/ir_send.rs` (R-1.b).
    /// Kept here as an inline sanity gate; the standalone test
    /// file follows in R-1.b per ir-v0.md §R1.
    #[allow(dead_code)]
    fn assert_send<T: Send>() {}

    #[test]
    fn ir_file_is_send() {
        // `tree_sitter::Tree: Send` but not `Sync`. IR holds
        // `Arc<Tree>`, so `Send` survives via `Arc<T> where T: Send`.
        assert_send::<IrFile>();
    }
}
