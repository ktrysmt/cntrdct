# Prior art for cntrdct detectors

Per design constraint P1, every detector MUST reference at least one peer-reviewed
publication or established benchmark. Each citation key here matches `Citation::key`
declared in `src/core.rs`.

The optional `Languages:` line records which languages each citation is grounded
in per `docs/spec/citations-policy.md`. Existing v0 entries are grandfathered as
Rust-grounded (the cited works study Java / C / C++ but the cross-cutting concept
papers transfer to Rust under clause (b)). New language-detector pairs added in
the M-series will list the target language explicitly when a qualifying citation
exists, or surface their absence via an explicit `unconfirmed:` annotation
pointing at the relevant `docs/surveys/` file.

## Layer 1 (Deterministic detectors)

### clone-drift

- `cordy-roy-icpc-2008` — J.R. Cordy, C.K. Roy, "The NiCad Clone Detector", ICPC 2008.
  Languages: Rust (grandfathered; original subjects were Java and C/C++).
- `bettenburg-msr-2009` — N. Bettenburg, W. Shang, W. Ibrahim, B. Adams, Y. Zou,
  A.E. Hassan, "An Empirical Study on Inconsistent Changes to Code Clones at the
  Release Level", MSR 2009.
  Languages: Rust (grandfathered).
- `krinke-icsm-2007` — J. Krinke, "A Study of Consistent and Inconsistent Changes to
  Code Clones", ICSM 2007.
  Languages: Rust (grandfathered).
- `assi-tosem-2025` — M. Assi, S. Hassan, Y. Zou, "Unraveling Code Clone Dynamics in
  Deep Learning Frameworks", ACM TOSEM 2025. DOI 10.1145/3721125. Independent
  peer-reviewed application of NiCad and SourcererCC to nine open-source Python
  deep-learning frameworks (TensorFlow, Paddle, PyTorch, Aesara, Ray, MXNet, Keras,
  Jax, BentoML); reports clone-coverage evolution and bug-fixing activity in
  cloned fragments across release histories.
  Languages: Python (clause (b) per docs/spec/citations-policy.md;
  see docs/surveys/clone-drift-python-2026-05.md).

### arg-swap

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically Extracting Implicit
  Programming Rules and Detecting Violations in Large Software Code", ESEC/FSE 2005.
  Languages: Rust (grandfathered; original subjects were C/C++).
- `rice-icse-2017` — A. Rice, E. Aftandilian, C. Jaspan, E. Johnston, M. Pradel,
  Y. Arroyo-Paredes, "Detecting Argument Selection Defects", ICSE 2017.
  Languages: Rust (grandfathered; original subjects were Java and C++).
- `allamanis-neurips-2021` — M. Allamanis, H. Jackson-Flux, M. Brockschmidt,
  "Self-Supervised Bug Detection and Repair", NeurIPS 2021. Introduces PyBugLab
  (Python implementation) and PyPIBugs (curated 2,374-bug Python evaluation
  corpus); argument swapping is one of four target bug classes.
  Languages: Python (clause (a) and (c) per docs/spec/citations-policy.md;
  see docs/surveys/arg-swap-python-2026-05.md).

### comment-code

- `tan-sosp-2007` — L. Tan, D. Yuan, G. Krishna, Y. Zhou, "/*iComment: Bugs or Bad
  Comments?*/", SOSP 2007.
  Languages: Rust (grandfathered; original subjects were C/C++ Linux kernel comments).
- `tan-pldi-2011` — L. Tan, Y. Zhou, Y. Padioleau, "aComment: Mining Annotations from
  Comments and Code to Detect Interrupt-related Concurrency Bugs", PLDI 2011.
  Languages: Rust (grandfathered).
- (comment-code Python coverage: unconfirmed; survey notes at
  docs/surveys/comment-code-python-2026-05.md)

### unreachable-after-terminator

- `hovemeyer-pugh-oopsla-2004` — D. Hovemeyer, W. Pugh, "Finding Bugs is Easy",
  OOPSLA 2004 (ACM SIGPLAN Notices 39(12)). Introduces the FindBugs "UR —
  Unreachable code" bug pattern category.
  Languages: Rust (grandfathered; FindBugs is a Java tool).
- `engler-sosp-2001` — D. Engler, D.Y. Chen, S. Hallem, A. Chou, B. Chelf, "Bugs as
  Deviant Behavior: A General Approach to Inferring Errors in Systems Code",
  SOSP 2001. Foundational work on control-flow contradictions as high-confidence
  anomaly signals.
  Languages: Rust (grandfathered; original subjects were the Linux kernel and C
  systems code).
- (unreachable-after-terminator Python coverage: unconfirmed; survey notes at
  docs/surveys/unreachable-after-terminator-python-2026-05.md)

### pr-miner

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically Extracting Implicit
  Programming Rules and Detecting Violations in Large Software Code", ESEC/FSE 2005.
  Languages: Rust (clause (b) per docs/spec/citations-policy.md; the frequent-itemset
  framing is language-agnostic and cntrdct's Rust port is the secondary application).
- (pr-miner Python coverage: unconfirmed; survey notes at
  docs/surveys/pr-miner-python-2026-05.md. v0.1 ships Python dispatch with
  `LanguageCitationStatus::Unconfirmed`, mirroring the comment-code precedent.)
- (Methodology reference for the Apriori miner: Agrawal & Srikant, VLDB 1994.
  Not listed as a Detector citation per citations-policy.md — the paper does not
  introduce the detector concept.)

### config-interaction

- `tartler-eurosys-2011` — B. Tartler, D. Lohmann, J. Sincero,
  W. Schröder-Preikschat, "Feature consistency in compile-time-configurable system
  software: facing the Linux 10,000 feature problem", EuroSys 2011. Canonical
  reference for the dead-block / inconsistent-feature anomaly class.
  Languages: Rust (the Rust `#[cfg(...)]` mechanism is the moral analogue of the
  C `#ifdef` / KConfig system the paper studies; Pattern B detector under
  `multilang-v0.md`).
- `nadi-icse-2014` — S. Nadi, T. Berger, C. Kästner, K. Czarnecki, "Mining
  configuration constraints: Static analyses and empirical results", ICSE 2014.
  Empirical evidence that contradictory cfg predicates recur in production code.
  Languages: Rust (grandfathered; original subjects were Linux / KConfig).

## Layer 2 (Statistical ranking)

- `kremenek-engler-sas-2003` — T. Kremenek, D. Engler, "Z-Ranking: Using Statistical
  Analysis to Counter the Impact of Static Analysis Approximations", SAS 2003.
  Languages: (general; methodological reference for the ranker, not detector-specific).
- `jung-kim-shin-yi-sas-2005` — Y. Jung, J. Kim, J. Shin, K. Yi, "Taming False Alarms
  from a Domain-Unaware C Analyzer by a Bayesian Statistical Post Analysis", SAS 2005.
  Languages: (general; methodological).
- `brown-cai-dasgupta-stat-sci-2001` — L.D. Brown, T.T. Cai, A. DasGupta, "Interval
  Estimation for a Binomial Proportion", Statistical Science 16(2), 101-133, 2001.
  DOI 10.1214/ss/1009213286. Source for Q-11's `n < 30` switching threshold between
  Wilson and Beta-prior credible-interval lower bounds, plus the boundary
  modification at `tp = 0` (§4) that `jeffreys_lower_95` applies.
  Languages: (general; methodological).
- `thulin-ejs-2014` — M. Thulin, "The cost of using exact confidence intervals for
  a binomial proportion", Electronic Journal of Statistics 8(1), 817-840, 2014.
  DOI 10.1214/14-EJS909. Independent argument for using a Beta-prior credible
  interval rather than the exact (Clopper-Pearson) interval at small `n`; provides
  the methodological grounding for Q-11's small-N branch.
  Languages: (general; methodological).

## Layer 3 (LLM adjudicator)

- `spiess-icse-2025` — C. Spiess et al., "Calibration and Correctness of Language
  Models for Code", ICSE 2025.
  Languages: (general; methodological — applies regardless of source language).
- `platt-1999` — J. Platt, "Probabilistic Outputs for Support Vector Machines and
  Comparisons to Regularized Likelihood Methods", Advances in Large Margin
  Classifiers (MIT Press), 1999. Methodology source for Q-12's post-hoc
  Platt-scaling step on LLM-emitted confidence values; applied per
  `(detector_id, anomaly_class)` cell on labelled adjudication corpora
  (see `docs/spec/llm-calibration-v0.md`).
  Languages: (general; methodological).
- `spiess-koohestani-sergeyuk-2025` — C. Spiess, P. Koohestani, A. Sergeyuk,
  "Verbalized Confidence in IDEs: A Large-Scale Empirical Study",
  arXiv:2510.22614, 2025. Empirical evidence (~24M IDE interactions) that
  verbalised LLM confidence is not better calibrated than the raw output;
  motivates Q-12's removal of the verbalised `calibration_tag` from the
  adjudicator prompt in favour of the post-hoc Platt fit.
  Languages: (general; methodological).
- `wataoka-2024` — K. Wataoka, T. Takahashi, R. Ri, "Self-Preference Bias
  in LLM-as-a-Judge", arXiv:2410.21819, 2024. Empirical evidence that LLM
  judges systematically prefer outputs from their own model family in
  pairwise comparison; motivates Q-13's cross-model agreement audit
  reported per `(detector_id, anomaly_class)` cell rather than averaged.
  See `docs/spec/cross-model-kappa-v0.md`.
  Languages: (general; methodological).
- `zheng-neurips-2023` — L. Zheng, W.-L. Chiang, Y. Sheng, S. Zhuang,
  Z. Wu, Y. Zhuang, Z. Lin, Z. Li, D. Li, E.P. Xing, H. Zhang, J.E.
  Gonzalez, I. Stoica, "Judging LLM-as-a-Judge with MT-Bench and Chatbot
  Arena", NeurIPS 36, 46595–46623, 2023. Establishes the LLM-as-judge
  baseline against human preferences and documents non-negligible verdict
  disagreement between equally capable judges; methodological grounding
  for Q-13's pairwise Cohen's κ surface.
  Languages: (general; methodological).

## Layer 4 (SARIF / severity)

- `oasis-sarif-2.1.0` — OASIS, "Static Analysis Results Interchange Format (SARIF)
  Version 2.1.0", 2020.
  Languages: (general; format specification).
- `ieee-1044-2009` — IEEE Std 1044-2009, "IEEE Standard Classification for Software
  Anomalies".
  Languages: (general; standard).
