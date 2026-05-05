#!/usr/bin/env python3
"""
Concatenate one or more cntrdct JSON findings arrays (each file produced
by `cntrdct scan --format json`) into a single JSON array on stdout.

Usage: merge_json.py <file1> [<file2> ...]

Used by the action wrapper when scanning multiple paths. Order is
preserved — findings from earlier files come first, matching the order
the user listed paths in the `paths:` input.
"""

import json
import sys
from typing import List


def main(argv: List[str]) -> int:
    if len(argv) < 2:
        print("usage: merge_json.py <file1> [<file2> ...]", file=sys.stderr)
        return 2
    out: list = []
    for path in argv[1:]:
        with open(path) as f:
            chunk = json.load(f)
        if not isinstance(chunk, list):
            print(f"merge_json.py: {path} is not a JSON array", file=sys.stderr)
            return 2
        out.extend(chunk)
    json.dump(out, sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
