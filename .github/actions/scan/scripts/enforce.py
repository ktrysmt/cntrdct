#!/usr/bin/env python3
"""
Enforce a fail-on policy: read a cntrdct JSON findings array from stdin
and exit non-zero when one or more findings meet the configured severity
threshold.

Usage: enforce.py {error|warning|never}
"""

import json
import sys

SEVERITY_SETS = {
    "never": set(),
    "warning": {"Error", "Warning"},
    "error": {"Error"},
}


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: enforce.py {error|warning|never}", file=sys.stderr)
        return 2
    mode = sys.argv[1]
    if mode not in SEVERITY_SETS:
        print(f"invalid mode: {mode}", file=sys.stderr)
        return 2

    bad = SEVERITY_SETS[mode]
    if not bad:
        return 0

    data = json.load(sys.stdin)
    offenders = [rf for rf in data if rf["finding"].get("raw_severity") in bad]
    if offenders:
        print(
            f"cntrdct: {len(offenders)} findings at or above '{mode}' severity",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
