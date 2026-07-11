//! `agy -p` CLI-shellout provider (Google Antigravity; the usage-cap
//! fallback for the default `scan --adjudicate` backend and the
//! cross-family confirmer in the Q-13 audit).
//!
//! Antigravity (`agy`) replaces the retired `gemini` CLI shellout: the
//! standalone Gemini CLI was folded into Antigravity upstream and is no
//! longer a distributed binary, so `gemini -p` no longer resolves on a
//! current install. `agy` is multi-model (Gemini / Claude / GPT-OSS), so
//! the self-preference guard (`candidate_llm::model_family`) classifies it
//! by the SELECTED MODEL string, not by the `agy-cli` provider id — the
//! shipped default forces a Gemini model so the provider stays
//! non-Anthropic (cross-family vs the `claude-cli` proposer / Anthropic
//! adjudicator).

use crate::core::{AdjudicationResult, Adjudicator, DetectorError, RankedFinding};

use super::{build_compact_prompt, parse_agy_cli_envelope, PromptDispatch};

// ---------- Constants ----------

/// Default executable name for Google Antigravity's CLI (replaces the
/// retired `gemini` shellout).
pub const AGY_CLI_PROGRAM: &str = "agy";
/// Default model passed to `agy --model`. A Gemini model is forced so
/// the provider stays non-Anthropic (cross-family vs the `claude-cli`
/// proposer); the literal is the display-name form `agy models` lists.
/// `Low` is the lightest variant — the gentlest on Antigravity's
/// free-tier rate limits. Override with `AGY_CLI_MODEL_OVERRIDE` (e.g. a
/// paid account bumping to `Gemini 3.5 Flash (Medium)`).
pub const AGY_CLI_MODEL: &str = "Gemini 3.5 Flash (Low)";
/// Provider id surfaced in cross-model audit logs.
pub const AGY_CLI_PROVIDER_ID: &str = "agy-cli";

/// Stronger closed-book directive prepended to the (flattened) prompt body
/// for `agy -p`. `agy` lacks a `--system-prompt` flag AND runs an agentic
/// persona: given a "review this finding" prompt it tries to *act*
/// (investigate / use tools) and, in `--print` mode with no tools, hangs or
/// returns an EMPTY response — the parse then fails and the verdict is
/// dropped. Folding [`CLI_SYSTEM_PROMPT`](super::CLI_SYSTEM_PROMPT) alone
/// was empirically insufficient; this forceful closed-book prefix (plus
/// single-line flattening in `dispatch`) makes `agy -p` emit the verdict
/// envelope. Verified against `Gemini 3.5 Flash (Low)`.
pub const AGY_SYSTEM_PROMPT: &str = "Output ONLY one line of valid JSON and nothing else. \
     Do not investigate, do not use tools, do not read files, do not call any tool, do not \
     produce prose, markdown, or explanation outside the JSON. This is a closed-book \
     classification task: answer using only the information in the prompt below, then stop.";

// ---------- AgyCliAdjudicator ----------

/// Shells out to Google Antigravity's `agy -p` (the multi-model CLI that
/// replaced the retired standalone `gemini` binary).
///
/// Auth is delegated to the user's existing `agy` login; the provider
/// holds no API key. Context auto-discovery (AGENTS.md / project memory)
/// is suppressed by spawning the subprocess with `current_dir =
/// <tempdir>`. Unlike `claude --print`, `agy -p` exposes neither
/// `--output-format json` nor `--system-prompt`: it prints the model's
/// raw text response to stdout, so [`Self::dispatch`] parses that text
/// directly ([`parse_agy_cli_envelope`]) and prepends [`AGY_SYSTEM_PROMPT`]
/// to the prompt body to stand in for the missing system-prompt flag.
///
/// `agy` is multi-model; the default [`AGY_CLI_MODEL`] forces a Gemini
/// model so `candidate_llm::model_family` classifies the provider as
/// `google` (non-Anthropic), keeping it a valid cross-family confirmer for
/// a `claude-cli` proposer / Anthropic adjudicator.
pub struct AgyCliAdjudicator {
    program: String,
    model: String,
    workdir: tempfile::TempDir,
}

impl AgyCliAdjudicator {
    /// Build a CLI adjudicator with default `program = "agy"` and the
    /// forced-Gemini [`AGY_CLI_MODEL`]. Allocates a tempdir used as the
    /// subprocess `cwd` so Antigravity picks up no project context.
    pub fn new() -> std::io::Result<Self> {
        let workdir = tempfile::tempdir()?;
        Ok(Self {
            program: AGY_CLI_PROGRAM.to_string(),
            model: AGY_CLI_MODEL.to_string(),
            workdir,
        })
    }

    /// Override the executable name / path. Used by tests to point at
    /// a stub script that emits a canned raw-text response.
    pub fn with_program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    /// Override the model passed to `agy --model`.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl PromptDispatch for AgyCliAdjudicator {
    fn provider_id(&self) -> &'static str {
        AGY_CLI_PROVIDER_ID
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError> {
        // No --system-prompt flag on `agy -p`; fold the persona-suppressing
        // instruction into the prompt body so the model still receives it.
        // Two empirically-required shapes for `agy -p` (verified against
        // `Gemini 3.5 Flash (Low)`):
        //  1. The forceful AGY_SYSTEM_PROMPT (not the weaker
        //     CLI_SYSTEM_PROMPT) — a verbose "evaluate this finding" prompt
        //     otherwise triggers agy's agentic persona, which hangs / returns
        //     an empty -p response instead of the verdict.
        //  2. A SINGLE-LINE prompt — a multi-line prompt (newlines in the
        //     `build_prompt` template / nested EVIDENCE_RAW JSON) trips the
        //     same agentic path. Collapsing all whitespace to single spaces
        //     keeps the prompt one line; JSON is whitespace-insensitive so
        //     the EVIDENCE_RAW block survives the flatten intact.
        let flattened = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
        let full_prompt = format!("{AGY_SYSTEM_PROMPT} {flattened}");
        // CRITICAL arg order: `agy`'s `--print` / `-p` takes the prompt as
        // its VALUE (it is not a boolean flag). `--model` must therefore come
        // FIRST, with the prompt as the token immediately after `--print` —
        // `agy --print --model <m> <prompt>` makes `--print` swallow
        // `"--model"` as the prompt and drops the real prompt as a stray
        // positional, which is why agy returned chatty/empty non-JSON.
        let output = std::process::Command::new(&self.program)
            .current_dir(self.workdir.path())
            .arg("--model")
            .arg(&self.model)
            .arg("--print")
            .arg(&full_prompt)
            .output()
            .map_err(|e| DetectorError::Config(format!("agy CLI invoke failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectorError::Config(format!(
                "agy -p exited with {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_agy_cli_envelope(stdout.as_ref()).map_err(DetectorError::from)
    }
}

impl Adjudicator for AgyCliAdjudicator {
    /// Run Layer 3 over `agy -p` subscription auth (no `ANTHROPIC_API_KEY`),
    /// with a non-Anthropic Gemini model so the verdict carries no
    /// self-preference bias against a `claude-cli` proposer. Used by
    /// `scan --adjudicate --adjudicate-via=agy-cli`.
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError> {
        // agy gets a COMPACT prompt, not the verbose `build_prompt` template.
        // The full template (labelled fields + a pretty-printed EVIDENCE_RAW
        // JSON block) reliably trips agy's agentic persona even when
        // flattened — agy hangs / returns empty. The compact form keeps the
        // decisive content (detector / message / location / inline evidence)
        // short enough that `agy -p` answers directly. Verified against
        // `Gemini 3.5 Flash (Low)`.
        let prompt = build_compact_prompt(finding);
        self.dispatch(&prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjudicator::test_support::write_stub_script;
    use crate::core::AdjudicationVerdict;

    #[test]
    fn agy_cli_dispatch_uses_print_model_flags_and_prepends_system_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        let argv_log = tmp.path().join("argv.log");
        // agy prints the raw text response (no JSON envelope).
        let stub_body = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\ncat <<'EOF'\n{{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.8,\"rationale\":\"r\"}}\nEOF\n",
            argv_log.display(),
        );
        let stub = write_stub_script(tmp.path(), "agy-stub", &stub_body);

        let adj = AgyCliAdjudicator::new()
            .unwrap()
            .with_program(stub.to_string_lossy().into_owned());
        let res = adj.dispatch("PROMPT-BODY").unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));

        let argv = std::fs::read_to_string(&argv_log).unwrap();
        let lines: Vec<&str> = argv.lines().collect();
        for flag in ["--print", "--model", AGY_CLI_MODEL] {
            assert!(
                lines.contains(&flag),
                "agy argv missing {}: got\n{}",
                flag,
                argv
            );
        }
        // LOAD-BEARING arg order: `--print` takes the prompt as its VALUE,
        // so `--model <m>` MUST come before `--print`, and the prompt MUST be
        // the token immediately after `--print`. Pinning this prevents the
        // regression where `--print` swallowed `--model` as the prompt and
        // agy returned chatty/empty non-JSON.
        let model_pos = lines.iter().position(|l| *l == AGY_CLI_MODEL).unwrap();
        let print_pos = lines.iter().position(|l| *l == "--print").unwrap();
        assert!(
            model_pos < print_pos,
            "agy --model must precede --print: got\n{}",
            argv
        );
        // The prompt (carrying the folded system prompt + body) is the arg
        // right after --print.
        let prompt_arg = lines[print_pos + 1];
        assert!(
            prompt_arg.contains(AGY_SYSTEM_PROMPT) && prompt_arg.contains("PROMPT-BODY"),
            "agy prompt (folded system prompt + body) must immediately follow --print: got\n{}",
            argv
        );
    }

    #[test]
    fn agy_cli_provider_id_is_agy_cli() {
        let adj = AgyCliAdjudicator::new().unwrap();
        assert_eq!(
            <AgyCliAdjudicator as PromptDispatch>::provider_id(&adj),
            "agy-cli"
        );
        assert_eq!(adj.model(), AGY_CLI_MODEL);
    }
}
