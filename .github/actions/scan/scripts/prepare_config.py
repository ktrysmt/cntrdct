#!/usr/bin/env python3
"""
Emit a per-path `cntrdct.toml` containing only a `[languages.*]` overlay.

Inputs:
  argv[1]: comma-separated canonical language names to keep ENABLED
           (e.g. "rust" or "rust,python").
  argv[2]: output path for the synthesized config.

The known-language universe is intentionally hard-coded here to match
`cntrdct-parsers::Language::all()`. New languages need a one-line update
to `KNOWN_LANGUAGES` below in lockstep with the parser crate; the
`action_language_universe_matches_all` integration test
(`tests/action_language_lockstep.rs`) fails the build if the two drift.

By design this script does not merge with a user-supplied `cntrdct.toml`:
the action wrapper rejects the combination of `config:` and a per-path
language hint up front, so the synthesized file is the SOLE config the
scan sees. That keeps the script free of any TOML parser dependency.
"""

import sys
from typing import List

KNOWN_LANGUAGES: List[str] = ["rust", "python", "typescript", "tsx", "go"]


def main(argv: List[str]) -> int:
    if len(argv) != 3:
        print("usage: prepare_config.py <lang_csv> <out_path>", file=sys.stderr)
        return 2
    lang_csv, out_path = argv[1], argv[2]
    hints = [s.strip() for s in lang_csv.split(",") if s.strip()]
    if not hints:
        print("prepare_config.py: empty language hint list", file=sys.stderr)
        return 2

    unknown = [h for h in hints if h not in KNOWN_LANGUAGES]
    if unknown:
        print(
            f"prepare_config.py: unknown language hints {unknown}; "
            f"known: {KNOWN_LANGUAGES}",
            file=sys.stderr,
        )
        return 2

    lines = []
    for lang in KNOWN_LANGUAGES:
        lines.append(f"[languages.{lang}]")
        lines.append("enabled = " + ("true" if lang in hints else "false"))
        lines.append("")

    with open(out_path, "w") as f:
        f.write("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
