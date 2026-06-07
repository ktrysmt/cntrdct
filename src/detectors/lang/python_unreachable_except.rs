//! python-unreachable-except detector — flag Python `except` handlers that can
//! never execute because an earlier handler in the same `try` already catches
//! the same exception class or one of its superclasses.
//!
//! This is the first language-specific detector under the post-R-1 two-tier
//! layout (`src/detectors/lang/`). It is Python-only by construction: the
//! analysis is grounded in the CPython built-in exception class hierarchy
//! (`data/python-builtin-exceptions.json`) plus same-file user-defined
//! exception classes. Like `rust_config_interaction`, it reads the raw
//! tree-sitter tree directly (Pattern B, ir-v0.md §F5) rather than the
//! language-agnostic IR.
//!
//! Spec: `docs/spec/unreachable-after-terminator-v0.md` §F4f.
//!
//! Scope (v0, ordering/subsumption only): a handler is flagged iff EVERY
//! exception type it catches is provably a subclass-or-equal of a type caught
//! by an earlier handler in the same `try`. Relationships that cannot be
//! resolved from (builtins ∪ same-file classes) are treated as INDETERMINATE
//! and never flagged — precision-first, so an imported/unknown exception type
//! produces no false positive. Body raise-set inference (handlers for
//! exceptions the `try` body cannot raise) and PEP 654 `except*` groups are
//! explicit non-goals; a `try` carrying an `except*` clause is skipped whole.

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::IrFile;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "hovemeyer-pugh-oopsla-2004",
        authors: "D. Hovemeyer, W. Pugh",
        title: "Finding Bugs is Easy",
        venue: "OOPSLA 2004 (ACM SIGPLAN Notices 39(12))",
        year: 2004,
        doi: Some("10.1145/1052883.1052895"),
        url: None,
        // General unreachable-code grounding (FindBugs "UR" pattern), not a
        // Python-specific study — declared general so it does not claim
        // per-language grounding on its own.
        languages: &[],
    },
    Citation {
        key: "de-padua-shang-icpc-2017",
        authors: "G. B. de Pádua, W. Shang",
        title: "Studying the Prevalence of Exception Handling Anti-Patterns",
        venue: "ICPC 2017 (IEEE/ACM 25th Int'l Conf. on Program Comprehension), pp. 328-331",
        year: 2017,
        doi: Some("10.1109/ICPC.2017.1"),
        url: None,
        // Defines the "Unreachable Handler" anti-pattern this detector
        // implements; the study's subjects are Java/C#, so the concept (not
        // Python) is what it grounds.
        languages: &[],
    },
];

/// Embedded CPython built-in exception hierarchy contract table.
/// Parsed once into a child -> direct-parent map. Compile-time `include_str!`
/// keeps `detect()` free of filesystem / network access (P3) and deterministic.
const TABLE_JSON: &str = include_str!("../../../data/python-builtin-exceptions.json");

#[derive(serde::Deserialize)]
struct ExcTable {
    classes: HashMap<String, ExcEntry>,
}

#[derive(serde::Deserialize)]
struct ExcEntry {
    parent: Option<String>,
}

static BUILTIN_PARENTS: OnceLock<HashMap<String, Option<String>>> = OnceLock::new();

fn builtin_parents() -> &'static HashMap<String, Option<String>> {
    BUILTIN_PARENTS.get_or_init(|| {
        let table: ExcTable =
            serde_json::from_str(TABLE_JSON).expect("embedded python exception table must parse");
        table
            .classes
            .into_iter()
            .map(|(name, entry)| (name, entry.parent))
            .collect()
    })
}

#[derive(Debug, Default)]
pub struct PythonUnreachableExcept;

impl PythonUnreachableExcept {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for PythonUnreachableExcept {
    fn id(&self) -> &'static str {
        "python-unreachable-except"
    }

    fn name(&self) -> &'static str {
        "Python Unreachable Except Handler"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Python]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = ctx
            .files
            .par_iter()
            .filter(|f| f.language == Language::Python)
            .flat_map_iter(|file| {
                let mut local = Vec::new();
                scan_file(file, &mut local);
                local
            })
            .collect();
        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
                .then(a.primary.start_col.cmp(&b.primary.start_col))
        });
        Ok(findings)
    }
}

fn scan_file(file: &IrFile, findings: &mut Vec<Finding>) {
    if file.parse_recovered {
        return;
    }
    let tree = file.raw_tree();
    let root = tree.root_node();
    // Same-file user-defined class hierarchy: name -> base-class names.
    let local = collect_class_bases(root, &file.source);
    walk(root, file, &local, findings);
}

/// One caught exception type within an `except` clause.
#[derive(Debug, Clone)]
struct CaughtType {
    /// Surface text of the type expression (e.g. `ValueError`, `socket.error`).
    /// Bare `except:` is represented as `BaseException` (the universal root).
    name: String,
    /// Location of the type expression (or the `except` clause for bare form).
    location: (u32, u32, u32, u32),
}

/// One `except` handler, in source order.
#[derive(Debug, Clone)]
struct Handler {
    types: Vec<CaughtType>,
    /// 1-based line of the `except` clause.
    line: u32,
    /// Location used as the finding primary when this handler is unreachable.
    report_location: (u32, u32, u32, u32),
}

fn walk(
    node: tree_sitter::Node,
    file: &IrFile,
    local: &HashMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    if node.kind() == "try_statement" {
        analyze_try(node, file, local, findings);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, file, local, findings);
    }
}

fn analyze_try(
    try_node: tree_sitter::Node,
    file: &IrFile,
    local: &HashMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    let mut handlers: Vec<Handler> = Vec::new();
    let mut cursor = try_node.walk();
    for child in try_node.children(&mut cursor) {
        match child.kind() {
            // PEP 654 exception groups are out of scope (v0): skip the whole
            // try so we never partially analyse a mixed/unsupported shape.
            "except_group_clause" => return,
            "except_clause" => {
                if let Some(h) = parse_except_clause(child, &file.source) {
                    handlers.push(h);
                }
            }
            _ => {}
        }
    }
    if handlers.len() < 2 {
        return;
    }

    // For each handler (from the second onward), it is unreachable iff every
    // type it catches is covered by some type caught by an earlier handler.
    for j in 1..handlers.len() {
        let child = &handlers[j];
        let mut covering: Option<(&CaughtType, &Handler, &CaughtType)> = None; // (child type, covering handler, covering type)
        let mut all_covered = true;
        for ct in &child.types {
            // Find an earlier caught type that covers `ct`.
            let mut found = None;
            'outer: for h in &handlers[..j] {
                for c in &h.types {
                    if covers(&c.name, &ct.name, local) {
                        found = Some((h, c));
                        break 'outer;
                    }
                }
            }
            match found {
                Some((h, c)) => {
                    if covering.is_none() {
                        covering = Some((ct, h, c));
                    }
                }
                None => {
                    all_covered = false;
                    break;
                }
            }
        }
        if !all_covered {
            continue;
        }
        let (child_ct, cover_handler, cover_type) =
            covering.expect("all_covered implies a covering type was recorded");

        let child_repr = if child.types.len() == 1 {
            child.types[0].name.clone()
        } else {
            let names: Vec<&str> = child.types.iter().map(|t| t.name.as_str()).collect();
            format!("({})", names.join(", "))
        };

        let message = format!(
            "except handler is unreachable; `{}` is already caught by `{}` on line {}",
            child_repr, cover_type.name, cover_handler.line
        );

        findings.push(Finding {
            detector_id: "python-unreachable-except".to_string(),
            primary: location_from_tuple(file, child.report_location),
            related: vec![location_from_tuple(file, cover_type.location)],
            message,
            raw_severity: Severity::Warning,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec!["hovemeyer-pugh-oopsla-2004", "de-padua-shang-icpc-2017"],
                raw: serde_json::json!({
                    "caught_type": child_repr,
                    "covering_type": cover_type.name,
                    "covering_line": cover_handler.line,
                    "matched_child_type": child_ct.name,
                }),
                // Python coverage is unconfirmed per docs/spec/citations-policy.md:
                // neither citation is grounded in a peer-reviewed Python study of
                // this anti-pattern (survey: docs/surveys/python-unreachable-except-python-2026-06.md).
                language_citation_status: LanguageCitationStatus::Unconfirmed,
            },
            origin: Default::default(),
        });
    }
}

/// Parse a single `except_clause` node into a [`Handler`]. Returns `None` only
/// for shapes we cannot interpret (which are skipped, not flagged).
fn parse_except_clause(clause: tree_sitter::Node, source: &str) -> Option<Handler> {
    let line = clause.start_position().row as u32 + 1;
    let clause_loc = node_tuple(clause);

    // The first NAMED child is either the caught-type expression or, for a
    // bare `except:`, the `block` body. The body block is always present.
    let type_node = {
        let mut cursor = clause.walk();
        let found = clause
            .named_children(&mut cursor)
            .find(|c| c.kind() != "block" && c.kind() != "comment");
        found
    };

    let Some(type_node) = type_node else {
        // Bare `except:` — catches everything (≡ BaseException).
        return Some(Handler {
            types: vec![CaughtType {
                name: "BaseException".to_string(),
                location: clause_loc,
            }],
            line,
            report_location: clause_loc,
        });
    };

    // `except E as e:` wraps the type in an as_pattern; unwrap to the type.
    let type_node = if type_node.kind() == "as_pattern" {
        let mut c = type_node.walk();
        let inner = type_node
            .named_children(&mut c)
            .find(|n| n.kind() != "as_pattern_target");
        inner.unwrap_or(type_node)
    } else {
        type_node
    };

    let types = if type_node.kind() == "tuple" {
        let mut c = type_node.walk();
        let collected: Vec<CaughtType> = type_node
            .named_children(&mut c)
            .filter(|n| n.kind() != "comment")
            .filter_map(|n| {
                n.utf8_text(source.as_bytes()).ok().map(|t| CaughtType {
                    name: t.trim().to_string(),
                    location: node_tuple(n),
                })
            })
            .collect();
        if collected.is_empty() {
            return None;
        }
        collected
    } else {
        let name = type_node
            .utf8_text(source.as_bytes())
            .ok()?
            .trim()
            .to_string();
        vec![CaughtType {
            name,
            location: node_tuple(type_node),
        }]
    };

    Some(Handler {
        types,
        line,
        report_location: node_tuple(type_node),
    })
}

/// Collect same-file `class Foo(Base, ...)` definitions into a
/// name -> base-class-names map. Only simple identifier / dotted-attribute
/// bases are recorded; keyword arguments (e.g. `metaclass=`) are ignored.
fn collect_class_bases(root: tree_sitter::Node, source: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    collect_class_bases_inner(root, source, &mut map);
    map
}

fn collect_class_bases_inner(
    node: tree_sitter::Node,
    source: &str,
    map: &mut HashMap<String, Vec<String>>,
) {
    if node.kind() == "class_definition" {
        if let Some(name) = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        {
            let mut bases = Vec::new();
            if let Some(supers) = node.child_by_field_name("superclasses") {
                let mut c = supers.walk();
                for arg in supers.named_children(&mut c) {
                    if !matches!(arg.kind(), "identifier" | "attribute" | "dotted_name") {
                        continue;
                    }
                    if let Ok(t) = arg.utf8_text(source.as_bytes()) {
                        bases.push(t.trim().to_string());
                    }
                }
            }
            map.insert(name.trim().to_string(), bases);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_class_bases_inner(child, source, map);
    }
}

/// Does a handler catching `ancestor` cover an exception of type `child`?
///
/// `BaseException` covers everything unconditionally (it is the root of every
/// exception). Otherwise `child` must be provably a subclass-or-equal of
/// `ancestor` via the builtin hierarchy chained with same-file user classes.
/// Unresolvable names yield `false` (precision-first: no over-claim).
fn covers(ancestor: &str, child: &str, local: &HashMap<String, Vec<String>>) -> bool {
    if ancestor == "BaseException" {
        return true;
    }
    is_subclass_or_equal(child, ancestor, local)
}

fn is_subclass_or_equal(child: &str, ancestor: &str, local: &HashMap<String, Vec<String>>) -> bool {
    let mut stack: Vec<String> = vec![child.to_string()];
    let mut seen: HashSet<String> = HashSet::new();
    while let Some(cur) = stack.pop() {
        if cur == ancestor {
            return true;
        }
        if !seen.insert(cur.clone()) {
            continue;
        }
        // Resolve direct parents: same-file user class bases take precedence,
        // then the builtin single-parent chain.
        if let Some(bases) = local.get(&cur) {
            for b in bases {
                stack.push(b.clone());
            }
            continue;
        }
        if let Some(Some(p)) = builtin_parents().get(&cur) {
            stack.push(p.clone());
        }
        // Unknown name (not in local, not in builtins): chain ends here.
    }
    false
}

fn node_tuple(node: tree_sitter::Node) -> (u32, u32, u32, u32) {
    let start = node.start_position();
    let end = node.end_position();
    (
        start.row as u32 + 1,
        start.column as u32 + 1,
        end.row as u32 + 1,
        end.column as u32 + 1,
    )
}

fn location_from_tuple(file: &IrFile, t: (u32, u32, u32, u32)) -> Location {
    Location {
        file: file.path.clone(),
        start_line: t.0,
        start_col: t.1,
        end_line: t.2,
        end_col: t.3,
    }
}
