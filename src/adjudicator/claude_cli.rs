//! `claude --print` CLI-shellout provider (Q-13; the DEFAULT
//! `scan --adjudicate` backend via `--adjudicate-via=claude-cli`).
//!
//! No HTTP path on the cntrdct side — auth and transport are delegated
//! to the user's existing `claude` login (OAuth / subscription), so no
//! `ANTHROPIC_API_KEY` is read.

use std::collections::HashMap;

use crate::core::{AdjudicationResult, Adjudicator, DetectorError, RankedFinding};

use super::{build_prompt, parse_claude_cli_envelope, PromptDispatch};

// ---------- Constants ----------

/// Q-13: default executable name for Claude Code's CLI.
pub const CLAUDE_CLI_PROGRAM: &str = "claude";
/// Default model for the `claude-cli` PROPOSER (Layer 0). Sonnet — semantic
/// Bound-B swap generation needs the stronger model.
pub const CLAUDE_CLI_MODEL: &str = "claude-sonnet-4-6";
/// Default model for the `claude-cli` ADJUDICATOR (Layer 3,
/// `scan --adjudicate-via=claude-cli`). Haiku — verdict is a binary
/// classification, so the cheaper / faster model suffices and is the
/// normal `claude -p` adjudication path. Overridable via
/// `CLAUDE_CLI_ADJUDICATE_MODEL_OVERRIDE`.
pub const CLAUDE_CLI_ADJUDICATE_MODEL: &str = "claude-haiku-4-5";
/// Q-13: provider id surfaced in cross-model audit logs.
pub const CLAUDE_CLI_PROVIDER_ID: &str = "claude-cli";

/// Q-13: minimal system prompt installed for the `claude` CLI provider.
/// The recipe assumes the CLI's default agentic persona is fully
/// overridden so the model receives essentially the user prompt only.
/// `claude --print` installs it via `--system-prompt`; `agy -p` has no
/// such flag and runs a stickier agentic persona, so
/// [`AgyCliAdjudicator`](super::AgyCliAdjudicator) uses the stronger
/// [`AGY_SYSTEM_PROMPT`](super::AGY_SYSTEM_PROMPT) instead.
pub const CLI_SYSTEM_PROMPT: &str = "You are evaluating a static analysis finding from cntrdct. \
     Respond only with the requested JSON object on a single line. \
     Do not call tools, do not read files, do not produce additional \
     prose.";

// ---------- ClaudeCliAdjudicator ----------

/// Q-13: shells out to Claude Code's `claude --print` with the
/// methodology-clean flag set documented in
/// `docs/spec/cross-model-kappa-v0.md` F2.
///
/// Auth is delegated to the user's existing `claude` login (OAuth /
/// subscription); the provider holds no API key. CLAUDE.md
/// auto-discovery is suppressed by spawning the subprocess with
/// `current_dir = <tempdir>`. The default flag set replaces Claude
/// Code's agentic persona with a minimal system prompt, disables
/// every built-in tool, and forces structured JSON output so the
/// inner verdict envelope is parseable byte-for-byte the same as
/// the HTTP path.
pub struct ClaudeCliAdjudicator {
    program: String,
    model: String,
    workdir: tempfile::TempDir,
}

impl ClaudeCliAdjudicator {
    /// Build a CLI adjudicator with default `program = "claude"` and
    /// `model = "claude-sonnet-4-6"`. Allocates a tempdir used as the
    /// subprocess `cwd` so CLAUDE.md auto-discovery picks up no
    /// project context.
    pub fn new() -> std::io::Result<Self> {
        let workdir = tempfile::tempdir()?;
        Ok(Self {
            program: CLAUDE_CLI_PROGRAM.to_string(),
            model: CLAUDE_CLI_MODEL.to_string(),
            workdir,
        })
    }

    /// Override the executable name / path. Used by tests to point at
    /// a stub script that emits a canned response envelope.
    pub fn with_program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    /// Override the model passed to `claude --model`.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl PromptDispatch for ClaudeCliAdjudicator {
    fn provider_id(&self) -> &'static str {
        CLAUDE_CLI_PROVIDER_ID
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError> {
        let output = std::process::Command::new(&self.program)
            .current_dir(self.workdir.path())
            .arg("--print")
            .arg("--model")
            .arg(&self.model)
            .arg("--system-prompt")
            .arg(CLI_SYSTEM_PROMPT)
            .arg("--tools")
            .arg("")
            .arg("--strict-mcp-config")
            .arg("--disable-slash-commands")
            .arg("--no-session-persistence")
            .arg("--output-format")
            .arg("json")
            .arg(prompt)
            .output()
            .map_err(|e| DetectorError::Config(format!("claude CLI invoke failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectorError::Config(format!(
                "claude --print exited with {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_claude_cli_envelope(stdout.as_ref()).map_err(DetectorError::from)
    }
}

impl Adjudicator for ClaudeCliAdjudicator {
    /// Run Layer 3 over `claude --print` subscription auth (no
    /// `ANTHROPIC_API_KEY`). Builds the standard finding prompt and routes
    /// it through the same `dispatch` the cross-model audit uses, so the
    /// verdict envelope is parsed identically to the HTTP path. Used by
    /// `scan --adjudicate --adjudicate-via=claude-cli`.
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError> {
        let prompt = build_prompt(finding, &HashMap::new());
        self.dispatch(&prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjudicator::test_support::write_stub_script;
    use crate::core::AdjudicationVerdict;

    #[test]
    fn claude_cli_dispatch_passes_methodology_clean_flags() {
        let tmp = tempfile::tempdir().unwrap();
        let argv_log = tmp.path().join("argv.log");
        let stub_body = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\ncat <<'EOF'\n{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"duration_ms\":1,\"result\":\"{{\\\"verdict\\\":\\\"Uncertain\\\",\\\"confidence\\\":0.5,\\\"rationale\\\":\\\"r\\\"}}\",\"session_id\":\"s\"}}\nEOF\n",
            argv_log.display()
        );
        let stub = write_stub_script(tmp.path(), "claude-stub", &stub_body);

        let adj = ClaudeCliAdjudicator::new()
            .unwrap()
            .with_program(stub.to_string_lossy().into_owned());
        let res = adj.dispatch("PROMPT-BODY").unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));

        let argv = std::fs::read_to_string(&argv_log).unwrap();
        // Pin every methodology-clean flag from the spec (F2).
        for flag in [
            "--print",
            "--model",
            CLAUDE_CLI_MODEL,
            "--system-prompt",
            CLI_SYSTEM_PROMPT,
            "--tools",
            "--strict-mcp-config",
            "--disable-slash-commands",
            "--no-session-persistence",
            "--output-format",
            "json",
            "PROMPT-BODY",
        ] {
            assert!(
                argv.lines().any(|l| l == flag),
                "claude argv missing {}: got\n{}",
                flag,
                argv
            );
        }
    }

    #[test]
    fn claude_cli_provider_id_is_claude_cli() {
        let adj = ClaudeCliAdjudicator::new().unwrap();
        assert_eq!(
            <ClaudeCliAdjudicator as PromptDispatch>::provider_id(&adj),
            "claude-cli"
        );
        assert_eq!(adj.model(), CLAUDE_CLI_MODEL);
    }

    #[test]
    fn cli_dispatch_surfaces_nonzero_exit_as_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        let stub = write_stub_script(
            tmp.path(),
            "fail-stub",
            "#!/bin/sh\necho 'auth required' >&2\nexit 1\n",
        );
        let adj = ClaudeCliAdjudicator::new()
            .unwrap()
            .with_program(stub.to_string_lossy().into_owned());
        let err = adj.dispatch("p").unwrap_err();
        match err {
            DetectorError::Config(msg) => {
                assert!(msg.contains("claude --print exited"), "got: {}", msg);
                assert!(msg.contains("auth required"), "stderr propagated: {}", msg);
            }
            other => panic!("expected Config error, got {:?}", other),
        }
    }
}
