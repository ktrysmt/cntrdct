//! SPDX license-expression filter for the analysis corpus.
//!
//! The empirical study restricts the corpus to permissively licensed crates so
//! that the resulting findings can be redistributed alongside the paper. The
//! allowlist mirrors `research/projects/A_1000_crate/README.md`: MIT, Apache-2.0,
//! BSD-3-Clause, ISC. The workspace `deny.toml` accepts a wider list for
//! transitive build dependencies, but corpus inclusion is intentionally
//! narrower because corpus crates are read and republished as evidence.
//!
//! This module hand-rolls a tiny SPDX-expression evaluator (OR / AND / WITH
//! plus parentheses, plus the legacy slash form) instead of taking on the
//! `spdx` crate as a dependency. The grammar fragment we care about is small,
//! we never need to render expressions, and we can fail closed on anything we
//! cannot parse.

/// Permissive licenses accepted into the analysis corpus.
///
/// Narrower than `deny.toml` on purpose: `deny.toml` governs *transitive*
/// dependencies of cntrdct itself, where weak copyleft like MPL-2.0 is fine.
/// The corpus contains crates we *redistribute snippets of* in the paper and
/// replication package, so we keep it to four well-understood permissive
/// licenses.
pub const DEFAULT_LICENSE_ALLOWLIST: &[&str] = &["MIT", "Apache-2.0", "BSD-3-Clause", "ISC"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseDecision {
    Accepted,
    Rejected,
    Missing,
}

/// Decide whether an optional SPDX expression is acceptable.
///
/// `None` and whitespace-only strings collapse to `Missing` so callers can
/// distinguish "rejected because copyleft" from "rejected because the crate
/// did not declare a license at all" — the two have different downstream
/// reporting (the former counts toward the license-distribution table, the
/// latter is excluded from analysis entirely).
pub fn license_decision(spdx: Option<&str>, allowlist: &[&str]) -> LicenseDecision {
    match spdx {
        None => LicenseDecision::Missing,
        Some(s) if s.trim().is_empty() => LicenseDecision::Missing,
        Some(s) => {
            if license_acceptable(s, allowlist) {
                LicenseDecision::Accepted
            } else {
                LicenseDecision::Rejected
            }
        }
    }
}

/// Evaluate an SPDX expression against an allowlist. Returns false on any
/// parse error (fail closed).
///
/// Handles:
/// - bare identifiers: `MIT`
/// - OR-chains: `MIT OR Apache-2.0`
/// - AND-chains: `MIT AND Apache-2.0`
/// - WITH-exceptions: `Apache-2.0 WITH LLVM-exception` (the exception name is
///   ignored; only the base identifier is checked against the allowlist)
/// - parenthesised groups: `(MIT OR Apache-2.0) AND BSD-3-Clause`
/// - legacy slash form: `MIT/Apache-2.0` (rewritten to `MIT OR Apache-2.0`)
///
/// Identifier comparison is ASCII case-insensitive: real Cargo.toml files
/// occasionally write `mit` or `apache-2.0`. The strict SPDX rule is
/// case-sensitive but corpus-fetch is for empirical analysis, not license
/// compliance, so we err on the side of accepting the few crates that mis-case
/// permissive identifiers rather than dropping them.
pub fn license_acceptable(spdx: &str, allowlist: &[&str]) -> bool {
    let normalized = spdx.replace('/', " OR ");
    let tokens = tokenize(&normalized);
    if tokens.is_empty() {
        return false;
    }
    let mut p = Parser { tokens, pos: 0 };
    match p.parse_or(allowlist) {
        Some(value) if p.pos == p.tokens.len() => value,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token<'a> {
    Ident(&'a str),
    Or,
    And,
    With,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'(' {
            out.push(Token::LParen);
            i += 1;
            continue;
        }
        if c == b')' {
            out.push(Token::RParen);
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'('
            && bytes[i] != b')'
        {
            i += 1;
        }
        let word = &s[start..i];
        if word.eq_ignore_ascii_case("OR") {
            out.push(Token::Or);
        } else if word.eq_ignore_ascii_case("AND") {
            out.push(Token::And);
        } else if word.eq_ignore_ascii_case("WITH") {
            out.push(Token::With);
        } else {
            out.push(Token::Ident(word));
        }
    }
    out
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos)
    }
    fn bump(&mut self) -> Option<Token<'a>> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_or(&mut self, allow: &[&str]) -> Option<bool> {
        let mut left = self.parse_and(allow)?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.bump();
            let right = self.parse_and(allow)?;
            left = left || right;
        }
        Some(left)
    }

    fn parse_and(&mut self, allow: &[&str]) -> Option<bool> {
        let mut left = self.parse_with(allow)?;
        while matches!(self.peek(), Some(Token::And)) {
            self.bump();
            let right = self.parse_with(allow)?;
            left = left && right;
        }
        Some(left)
    }

    fn parse_with(&mut self, allow: &[&str]) -> Option<bool> {
        let left = self.parse_atom(allow)?;
        if matches!(self.peek(), Some(Token::With)) {
            self.bump();
            // Consume one identifier as the exception name and discard it.
            match self.bump() {
                Some(Token::Ident(_)) => {}
                _ => return None,
            }
        }
        Some(left)
    }

    fn parse_atom(&mut self, allow: &[&str]) -> Option<bool> {
        match self.bump()? {
            Token::LParen => {
                let inner = self.parse_or(allow)?;
                match self.bump() {
                    Some(Token::RParen) => Some(inner),
                    _ => None,
                }
            }
            Token::Ident(id) => Some(allow.iter().any(|a| a.eq_ignore_ascii_case(id))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow() -> &'static [&'static str] {
        DEFAULT_LICENSE_ALLOWLIST
    }

    #[test]
    fn bare_identifier_in_allowlist_accepts() {
        assert!(license_acceptable("MIT", allow()));
        assert!(license_acceptable("Apache-2.0", allow()));
        assert!(license_acceptable("BSD-3-Clause", allow()));
        assert!(license_acceptable("ISC", allow()));
    }

    #[test]
    fn bare_identifier_outside_allowlist_rejects() {
        assert!(!license_acceptable("GPL-3.0", allow()));
        assert!(!license_acceptable("AGPL-3.0", allow()));
        assert!(!license_acceptable("LGPL-2.1", allow()));
        assert!(!license_acceptable("MPL-2.0", allow()));
    }

    #[test]
    fn or_accepts_when_any_branch_matches() {
        assert!(license_acceptable("MIT OR Apache-2.0", allow()));
        assert!(license_acceptable("Apache-2.0 OR MIT", allow()));
        // Mixed: GPL on one side, MIT on the other — recipient may pick MIT.
        assert!(license_acceptable("MIT OR GPL-3.0", allow()));
        assert!(license_acceptable("GPL-3.0 OR MIT", allow()));
    }

    #[test]
    fn or_rejects_when_no_branch_matches() {
        assert!(!license_acceptable("GPL-3.0 OR AGPL-3.0", allow()));
    }

    #[test]
    fn and_requires_every_atom_in_allowlist() {
        assert!(license_acceptable("MIT AND Apache-2.0", allow()));
        // AND with a single non-permissive atom fails.
        assert!(!license_acceptable("MIT AND GPL-3.0", allow()));
    }

    #[test]
    fn with_clause_uses_base_license() {
        assert!(license_acceptable(
            "Apache-2.0 WITH LLVM-exception",
            allow()
        ));
        assert!(!license_acceptable(
            "GPL-3.0 WITH Classpath-exception-2.0",
            allow()
        ));
    }

    #[test]
    fn parens_group_and_or() {
        assert!(license_acceptable(
            "(MIT OR Apache-2.0) AND BSD-3-Clause",
            allow()
        ));
        assert!(!license_acceptable(
            "(MIT OR Apache-2.0) AND GPL-3.0",
            allow()
        ));
        assert!(license_acceptable("(MIT)", allow()));
    }

    #[test]
    fn legacy_slash_treated_as_or() {
        assert!(license_acceptable("MIT/Apache-2.0", allow()));
        assert!(license_acceptable("MIT/GPL-3.0", allow()));
        assert!(!license_acceptable("GPL-3.0/AGPL-3.0", allow()));
    }

    #[test]
    fn ascii_case_insensitive_match() {
        assert!(license_acceptable("mit", allow()));
        assert!(license_acceptable("apache-2.0 or MIT", allow()));
    }

    #[test]
    fn malformed_input_rejects() {
        assert!(!license_acceptable("", allow()));
        assert!(!license_acceptable("(MIT", allow()));
        assert!(!license_acceptable("MIT OR", allow()));
        assert!(!license_acceptable("OR MIT", allow()));
        assert!(!license_acceptable("MIT WITH", allow()));
    }

    #[test]
    fn license_decision_distinguishes_missing_from_rejected() {
        assert_eq!(license_decision(None, allow()), LicenseDecision::Missing);
        assert_eq!(
            license_decision(Some(""), allow()),
            LicenseDecision::Missing
        );
        assert_eq!(
            license_decision(Some("   "), allow()),
            LicenseDecision::Missing
        );
        assert_eq!(
            license_decision(Some("MIT"), allow()),
            LicenseDecision::Accepted
        );
        assert_eq!(
            license_decision(Some("GPL-3.0"), allow()),
            LicenseDecision::Rejected
        );
    }
}
