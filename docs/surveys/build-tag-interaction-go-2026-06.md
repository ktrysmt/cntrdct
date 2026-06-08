# Survey: build-tag-interaction (Go) — 2026-06

Detector: `build-tag-interaction-go` (Go `//go:build` constraint
contradiction). Survey per `docs/spec/citations-policy.md` to decide the
`LanguageCitationStatus` for Go.

Outcome: UNCONFIRMED. No peer-reviewed, Go-subject study grounds the
"unsatisfiable build constraint" anti-pattern specifically. The concept
is grounded by configuration-variability work on C / Linux KConfig; the
detector ships Go with `LanguageCitationStatus::Unconfirmed`, carrying
the concept keys.

## Concept grounding (accepted, general)

- `tartler-eurosys-2011` — Tartler et al., "Feature consistency in
  compile-time-configurable system software: facing the Linux 10,000
  feature problem", EuroSys 2011. Defines the dead-block / inconsistent-
  feature anomaly class over `#ifdef` / KConfig. A contradictory
  `//go:build` constraint is the same always-false configuration
  predicate. Subjects: C / Linux. Grounds the concept, not Go.
- `nadi-icse-2014` — Nadi et al., "Mining configuration constraints:
  Static analyses and empirical results", ICSE 2014. Empirical evidence
  that contradictory configuration predicates recur in real systems.
  Subjects: Linux / KConfig. Grounds the concept, not Go.

These are the same keys `config-interaction` (Rust) uses; the
build-constraint contradiction is the cross-language instance of the
class.

## Candidates considered and rejected for Go-specific grounding

- Go documentation / `go/build` and `cmd/go` source (the
  `//go:build` constraint parser, `go vet`'s `buildtag` analyzer): these
  are authoritative tooling, not peer-reviewed publications, and the
  `buildtag` vet check targets `//go:build` / `// +build` MISMATCH and
  malformed syntax, not semantic unsatisfiability (`X && !X`). Does not
  satisfy clause (a) for Go.
- Go-subject empirical studies surveyed in the R-3 Go pilot
  (`docs/surveys/*-go-2026-06.md`: Go-Clone ISSTA 2019, Tu et al. ASPLOS
  2019 / GoBench, the JSS 2026 Go-linters assessment): none studies build
  constraints or configuration contradictions in Go.
- Feature-interaction / variability-bug literature beyond Tartler / Nadi
  (e.g. Abal et al. "42 variability bugs in the Linux kernel"): still
  C / Linux subjects; reinforces the concept but adds no Go grounding.

## Decision

Ship Go with `Unconfirmed`. Revisit if a peer-reviewed study of Go build-
constraint defects (or configuration-variability defects in Go) appears.
