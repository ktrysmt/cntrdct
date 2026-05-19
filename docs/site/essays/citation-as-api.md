---
layout: page
title: "The Linter that Cites Its Sources"
description: "What happens when a static analyser's findings must each carry a peer-reviewed citation."
date: 2026-05-03
permalink: /essays/citation-as-api/
---

Some weeks ago I noticed that the static-analysis tools I use during
code review fall into two camps. The traditional linters — clippy,
ESLint, Pylint — emit findings I can chase to a documentation page that
explains the rule and, if I dig further, to the GitHub issue or paper
that motivated it. The AI-generated review tools emit plausible-sounding
paragraphs and evaporate when I ask for the evidence. The first kind has
a trust problem when its rules are wrong. The second kind has a trust
problem when its rules are right.

I built `cntrdct`, an evidence-based contradiction linter for Rust,
partly to see what would happen if I made the second kind impossible by
construction. Every finding `cntrdct` emits carries a citation key that
maps to a specific peer-reviewed paper. Detectors without citations
cannot register. The framework rejects them at startup. This single
architectural choice has consequences I hadn't anticipated when I
started, and they are interesting enough to be worth writing about.

## The thesis

One sentence:

> Treating peer-reviewed citation as an enforced API of a static
> analyser produces design constraints that ripple through the entire
> tool architecture, and those constraints are useful in their own
> right.

"You should cite your sources" is the obvious version of this. The
interesting version is what citation enforcement does to the rest of the
tool — to detectors, to LLM integration, to empirical calibration, to
long-term maintenance. The rest of this post is the second version.

## P1: Citation as an enforced API

Every detector in `cntrdct` implements a trait whose `citations()`
method returns a non-empty slice of `Citation` values. The framework's
`register_detector` function asserts non-emptiness at runtime. There's
no clever metaprogramming here — just a startup check — but the
practical consequence is that adding a new detector requires finding a
citation before the detector can be compiled into the binary.

```rust
pub trait Detector {
    fn id(&self) -> &'static str;
    fn citations(&self) -> &'static [Citation];
    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, ...>;
    // ...
}

pub fn register_detector(d: &dyn Detector) -> Result<(), DetectorError> {
    if d.citations().is_empty() {
        return Err(DetectorError::Config(
            "P1 violation: detector cites no prior art".into()
        ));
    }
    // ...
}
```

This sounds cosmetic. It is not.

I noticed myself reaching for citations *earlier* in the development
cycle. When I sketched the `comment-code` detector — which flags doc
comments that claim behaviour the implementation doesn't exhibit — my
first move was to look up Lin Tan's iComment paper from SOSP 2007. The
paper showed that comment-code mismatch is better decomposed into
specific patterns than treated as a single generic check; iComment
itself focused on synchronization annotations in C systems code. My
Rust port needed different patterns (Result claims, panic claims,
deprecated claims), but the methodological insight — pattern-by-pattern,
not catch-all — carried over. My naive pre-citation sketch was a
single generic mismatch detector. The citation-first workflow saved me
from shipping it.

Compare this with the usual order: implement the detector, run it
against a few examples, ship it, write a blog post about the
heuristic, perhaps cite a paper retroactively. The tooling produced
under that workflow ages badly. The detector becomes a folk artifact,
divorced from the literature it half-remembers.

I am not the first person to argue that linters should cite. What I'm
claiming is that *enforcing* the citation at the type level — making
the detector unconstructable without a citation — changes contributor
behaviour in measurable ways, even when the contributor is just me at
11 pm.

## P3: LLM containment as an architectural boundary

`cntrdct`'s second design constraint is that only the `Adjudicator`
layer is permitted to call out to a language model. Detectors and
rankers are network-free, deterministic, and compile against a stub
`Adjudicator` for tests. An LLM verdict is a separable add-on, not a
load-bearing component.

The motivation is reproducibility. Tools that interleave deterministic
and LLM logic produce findings that depend on the model version, the
prompt template, the API endpoint, and a host of other variables not
under version control. Six months from now, a user running the same
tool against the same code can get a different answer, and there's no
way to tell which run was right.

By containing the LLM in a single architectural layer, `cntrdct` makes
a different deal. The deterministic detectors produce the same findings
forever. The ranker produces the same ordering forever, given the same
priors. Only the LLM verdict drifts, and it's clearly labelled as a
verdict, not a finding. A reviewer who wants reproducibility runs with
`--no-adjudicate`. A reviewer who wants the AI-augmented workflow runs
`--adjudicate` and sees precisely which findings got an LLM-touched
assessment, distinct from the underlying anomaly detection.

The boundary costs something. The LLM cannot reach into the detector
to suggest a fix or refine the detection rule. Some clever workflows
become impossible. I don't mind. The workflows we lose are roughly the
workflows that produce irreproducible output, which is the class I
wanted to lose anyway.

## P4: Empirical priors as falsifiability

The third constraint is that the ranker's priors come from a labelled
corpus, not from hardcoded confidence figures. I don't pick a "warning
severity" out of the air. I run the detector against a corpus, count
true positives and false positives, and compute a Wilson 95 percent
lower bound on precision. That number becomes the prior. The ranker
multiplies it by a sibling-count factor and orders findings by the
product.

A concrete example. The `arg-swap` detector currently ships with a
hand-curated regression corpus where its precision is 1.0. That's an
upper bound — a sanity check, not a finding. When I run the same
detector against a separately collected corpus of public Rust code, I
expect precision to drop. If it drops to, say, 0.7 with a Wilson lower
bound of 0.55, that becomes the prior. Findings get re-ordered. Users
see arg-swap rated less confident than `unreachable-after-terminator`
(which is structural and stays near 1.0). The ranker is *publishing
its own track record*.

The contrast is with the hardcoded-severity world, where someone made
a decision in 2017 about how confident to be in a rule, the underlying
corpus has shifted twice since, and nobody notices because the severity
string in the codebase is still `Warning`. Tools that ship with
hardcoded confidence are, in a real sense, lying about their precision.
They're reporting a guess from years ago as if it were a measurement of
today.

For everyday use, the point is just that the linter publishes its
own measured precision against a labelled corpus that ships with
the repo, and ships fresh numbers on every release.

## The compound payoff: auditability

Each of P1, P3, P4 looks small in isolation. The interesting result is
what they yield together.

A reviewer who picks up a `cntrdct` finding can audit it end to end.
They can read the citation that justifies the detector's existence.
They can read the deterministic source code that decides when to fire.
They can check the empirical prior that ordered this finding above
another. They can re-run the LLM verdict if one was attached and see
whether the model agrees with the deterministic layer.

A worked example. The detector emits a finding for `arg-swap` on
`copy(src, dst)` at `src/io.rs:42`. The reviewer follows the finding's
`citation_keys` to `rice-icse-2017`, opens the paper, confirms that
parameter-name vs argument-name matching is the published technique.
They open the detector's source, see the matching rule (lower-cased
identifier comparison, simple identifiers only, exactly two
parameters). They check the ranker output and see that this finding
has `wilson_lower = 0.55`, computed from a corpus they can re-download.
They run with `--adjudicate` and see the LLM verdict, with its
confidence and rationale, attached separately. Every layer is
checkable. Nothing is opaque.

There are no opaque thresholds. There is no "internal severity" the
team uses but doesn't expose. There is no hidden prompt the LLM saw
but the reviewer didn't. The whole tool is, in a phrase,
reading-the-paper-equivalent — the same transparency a reviewer who
consulted the founding paper would have, plus runnable code.

This is the property I find genuinely useful. Most tools I've used
over the last decade have one or two opaque components. Either the
heuristic is undocumented, or the severity is unjustified, or the LLM
is unconstrained. `cntrdct` makes the audit cheap because every layer
is checkable.

## The open question: citation decay

Here's the part I haven't solved.

Citations age. Papers get superseded. Findings get retracted. A 2008
clone-detection paper that motivates a detector in 2026 is making
claims that have since been refined, criticised, or replaced. The
citation in the detector's `citations()` method does not know this.
It points at a paper that the field may have moved past.

Take Cordy and Roy's NiCad (ICPC 2008), one of `cntrdct`'s
clone-drift citations. NiCad is sound published work. It's also
nearly two decades old, and the clone-detection field has produced
SourcererCC, Oreo, and various ML-based descendants since. The
detector cites NiCad because the simplification I shipped is faithful
to NiCad, not to the descendants. But a reader landing on the
citation today will see a 2008 paper and reasonably ask whether the
field has moved on. The citation graph is not telling them.

I don't currently have an answer for this. The naive solution — track
each citation's "freshness" with a metadata field — works for
retractions but not for graceful supersession. The clever solution — a
meta-detector that flags detectors whose citations have been overtaken
in the literature — sounds great in a workshop talk but requires
citation-graph data that doesn't exist for software-engineering papers
in any clean form.

I think this is a real research question. If "citation as API" is to
scale beyond a hobby project to, say, an industrial team's codebase,
the temporal evolution of the citation graph has to be addressable.
Three sub-questions in particular:

1. How does a tool detect that one of its citations has been
   superseded, given that supersession is rarely declared explicitly
   in software-engineering literature?
2. What does the right "freshness" UI look like — a per-finding
   confidence haircut, an explicit warning, a mandatory citation
   refresh, something else?
3. When two citations contradict each other (one paper claims a
   pattern is a bug, a later paper claims it's idiomatic), how
   should the tool decide which to honour, and how should it
   surface the contradiction to the user?

I'd be interested in hearing from anyone who has thought about this.

## Where this leaves things

`cntrdct` is open source at
[github.com/ktrysmt/cntrdct](https://github.com/ktrysmt/cntrdct). It's
a Rust tool for Rust code, with five Layer 1 detectors and a four-layer
architecture: deterministic detectors, statistical ranker, optional LLM
adjudicator, SARIF emitter. The detectors aren't novel — they're
faithful Rust ports of patterns from the literature, which is the
point. The architecture is the contribution.

If you're building static analysis tooling in 2026, especially tooling
that integrates with LLMs, I think the constraints I've described —
type-level citation enforcement, LLM containment in a single layer,
empirical priors instead of guesses — are worth considering. They make
the tool harder to write. They make it easier to trust.
