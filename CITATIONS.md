# Prior art for cntrdct detectors

Per design constraint P1, every detector MUST reference at least one peer-reviewed
publication or established benchmark. Each citation key here matches `Citation::key`
declared in `crates/core/src/lib.rs`.

## Layer 1 (Deterministic detectors)

### clone-drift

- `cordy-roy-icpc-2008` — J.R. Cordy, C.K. Roy, "The NiCad Clone Detector", ICPC 2008.
- `bettenburg-msr-2009` — N. Bettenburg, W. Shang, W. Ibrahim, B. Adams, Y. Zou,
  A.E. Hassan, "An Empirical Study on Inconsistent Changes to Code Clones at the
  Release Level", MSR 2009.
- `krinke-icsm-2007` — J. Krinke, "A Study of Consistent and Inconsistent Changes to
  Code Clones", ICSM 2007.

### arg-swap

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically Extracting Implicit
  Programming Rules and Detecting Violations in Large Software Code", ESEC/FSE 2005.
- `rice-icse-2017` — A. Rice, E. Aftandilian, C. Jaspan, E. Johnston, M. Pradel,
  Y. Arroyo-Paredes, "Detecting Argument Selection Defects", ICSE 2017.

### comment-code

- `tan-sosp-2007` — L. Tan, D. Yuan, G. Krishna, Y. Zhou, "/*iComment: Bugs or Bad
  Comments?*/", SOSP 2007.
- `tan-pldi-2011` — L. Tan, Y. Zhou, Y. Padioleau, "aComment: Mining Annotations from
  Comments and Code to Detect Interrupt-related Concurrency Bugs", PLDI 2011.

### unreachable-after-terminator

- `hovemeyer-pugh-oopsla-2004` — D. Hovemeyer, W. Pugh, "Finding Bugs is Easy",
  OOPSLA 2004 (ACM SIGPLAN Notices 39(12)). Introduces the FindBugs "UR —
  Unreachable code" bug pattern category.
- `engler-sosp-2001` — D. Engler, D.Y. Chen, S. Hallem, A. Chou, B. Chelf, "Bugs as
  Deviant Behavior: A General Approach to Inferring Errors in Systems Code",
  SOSP 2001. Foundational work on control-flow contradictions as high-confidence
  anomaly signals.

### config-interaction

- `tartler-eurosys-2011` — B. Tartler, D. Lohmann, J. Sincero,
  W. Schröder-Preikschat, "Feature consistency in compile-time-configurable system
  software: facing the Linux 10,000 feature problem", EuroSys 2011. Canonical
  reference for the dead-block / inconsistent-feature anomaly class.
- `nadi-icse-2014` — S. Nadi, T. Berger, C. Kästner, K. Czarnecki, "Mining
  configuration constraints: Static analyses and empirical results", ICSE 2014.
  Empirical evidence that contradictory cfg predicates recur in production code.

## Layer 2 (Statistical ranking)

- `kremenek-engler-sas-2003` — T. Kremenek, D. Engler, "Z-Ranking: Using Statistical
  Analysis to Counter the Impact of Static Analysis Approximations", SAS 2003.
- `jung-kim-shin-yi-sas-2005` — Y. Jung, J. Kim, J. Shin, K. Yi, "Taming False Alarms
  from a Domain-Unaware C Analyzer by a Bayesian Statistical Post Analysis", SAS 2005.

## Layer 3 (LLM adjudicator)

- `spiess-icse-2025` — C. Spiess et al., "Calibration and Correctness of Language
  Models for Code", ICSE 2025.

## Layer 4 (SARIF / severity)

- `oasis-sarif-2.1.0` — OASIS, "Static Analysis Results Interchange Format (SARIF)
  Version 2.1.0", 2020.
- `ieee-1044-2009` — IEEE Std 1044-2009, "IEEE Standard Classification for Software
  Anomalies".
