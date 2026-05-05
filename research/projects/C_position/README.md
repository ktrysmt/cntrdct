# Track C — position paper on evidence-based linter design

## The argument

A static analysis tool's findings carry implicit authority. Reviewers
trust them, CI gates them, downstream automation acts on them. In the
LLM-generated code review world that authority is increasingly
ambiguous: AI tools produce plausible-sounding feedback without
traceable grounding. The cntrdct project takes a contrarian position.
Citation to peer-reviewed prior art is treated as a first-class artefact
of every detector. The architectural consequences of that single
constraint are not obvious, and they are the subject of this paper.

The thesis, as a sentence:

  Treating peer-reviewed citation as an enforced API of a static
  analyser produces design constraints that ripple through the entire
  tool architecture, and those constraints are useful in their own
  right.

## What separates this from a blog post

A blog post can stop at "tools should cite their sources". The paper
needs to make non-obvious claims that survive scrutiny. Candidate
non-obvious claims:

C1. Citation enforcement at the type level (P1 in cntrdct: detectors
without `citations()` cannot register) is not merely cosmetic. It
forces detector authors to pre-commit to an evidentiary standard
before writing detection logic. The order matters: the empirical
question precedes the implementation.

C2. Constraining LLM invocation to a single architectural layer (P3:
only Adjudicator may call out) decouples reproducibility from any
specific model version. Detectors and rankers remain deterministic
and version-stable; the LLM lives in a swappable component. This
matters for tools that outlive a particular foundation model.

C3. Empirical priors (P4: priors come from labelled corpora, never
from hardcoded confidence figures) make the linter falsifiable. A
detector whose precision drops on a new corpus surfaces in the
ranker output rather than being hidden behind a static "warning
severity" string.

C4. The combination of P1 (citations), P3 (LLM containment), and P4
(empirical priors) yields a tool whose individual claims are
auditable by an outside reviewer with the source code and the
corpus alone — no internal-only labels, no opaque thresholds.

C5. Open question: citation graphs decay. How should an
evidence-based linter behave when its founding paper is superseded,
contradicted, or retracted? Possible answers (paper-introduces-
freshness-metadata, paper-introduces-conflicting-citation-resolution,
paper-introduces-meta-detector-on-citation-staleness) are sketched.

If the paper can defend C1-C4 with concrete examples drawn from
cntrdct's implementation and frame C5 as a research direction, it
crosses the bar from blog material to essay-class contribution.

## Existing infrastructure

cntrdct itself is the worked example. Useful to point at:

- `crates/core/src/lib.rs` — the `register_detector` function that
  enforces P1 at startup.
- `crates/cli/tests/citations_consistency.rs` — the test that holds
  `CITATIONS.md` and `Detector::citations()` in lock-step.
- `crates/adjudicator-llm/` — the only network-touching crate;
  `crates/calibration` and the detector crates are network-free.
- `crates/calibration/` — empirical priors via Wilson lower bound
  and Laplace posterior.
- `prereg/2026-05-03-osf-prereg.md` — the internal governance
  prereg as an example of P2 (preregistration metadata).
- `CITATIONS.md` — the bibliography that sits next to the code.

The paper benefits from cntrdct existing; it is not blocked on more
implementation.

## Stage gates

Stage 1 — blog post (1-2 weekends).

- Target length: 2000-3000 words.
- Audience: Rust + SE practitioners.
- Distribution: personal blog, then submission to lobste.rs and
  Hacker News. Watch reactions for signal on whether C1-C5 read as
  novel or obvious.

Stage 2 — refined essay (1-2 months part-time, only if Stage 1
generates substantive responses).

- Target length: 8-12 pages, double-column.
- Strengthen each claim with a concrete cntrdct example.
- Add a "non-goals" section that pre-empts the most common
  objection (citation requirements scale poorly to teams without
  research access).
- Open questions section for C5 expanded to 3-5 distinct questions,
  each with a sketched empirical study that could answer it.

Stage 3 — submission (1-2 weeks for the submission cycle).

- Target venue: Onward! Essays at SPLASH (https://2024.splashcon.org/track/splash-2024-onward-essays).
  Alternative: CACM Viewpoint (~3000 words, longer review cycle, no
  guaranteed acceptance), IEEE Software practitioner column.

## Effort estimate

- Stage 1: 1-2 weekends.
- Stage 2: 1-2 months part-time.
- Stage 3: 1-2 weeks submission cycle plus revision rounds.

Total best case: 3 months. Skip Stage 2 / 3 entirely if Stage 1's
reception suggests the argument lands but doesn't deepen further.

## Venue targets

In descending fit:

- Onward! Essays at SPLASH — explicitly receptive to argumentative,
  personal, polemical essays. Best fit.
- CACM Viewpoint — practitioner-shaped argument, broad audience.
- IEEE Software practitioner column — short, frequent, fast.
- ICSE NIER (New Ideas and Emerging Results, 4 pages) — possible if
  the paper is reframed around an open research direction with a
  small empirical pilot. Without empirical content NIER is unlikely.

## First concrete step (done 2026-05-03)

Stage 1 draft authored. The essay was moved out of this scratch
directory into the GitHub Pages source tree as the canonical home,
since the project decided against a personal-blog channel:

- Source: `docs/site/essays/citation-as-api.md`
- Site source: `docs/site/_config.yml`, `docs/site/index.md`
- Deploy workflow: `.github/workflows/pages.yml`
- Public URL (after Pages is enabled in repository settings):
  `https://ktrysmt.github.io/cntrdct/essays/citation-as-api/`

The earlier scratch file `projects/C_position/blog-draft-v0.md` is
superseded and can be removed. To delete:

```sh
rm projects/C_position/blog-draft-v0.md
```

## Next concrete steps

1. Enable GitHub Pages in repository Settings → Pages → Source =
   "GitHub Actions". One-time manual click; the existing workflow
   runs the rest automatically.
2. Push to `master`, watch the workflow deploy, confirm the public
   URL renders.
3. Distribute. Submit to lobste.rs, r/rust, Hacker News with the
   GitHub Pages URL. Watch reactions for signal on whether claims
   C1-C5 read as novel or obvious.
4. If reception is strong, proceed to Stage 2 (Onward! Essays
   submission). If lukewarm, leave the essay as-is and pick up
   tracks A or B.

## Dependencies on other tracks

- The blog post requires the OSS repository to be public, so it is
  blocked on the practical track's Tier 1 OSS readiness items
  (CI, README polish, crates.io publish).
- Track A or B as companion empirical work would strengthen Stage 2,
  but Stage 1 (blog) does not require them.
- C does not block A or B.
