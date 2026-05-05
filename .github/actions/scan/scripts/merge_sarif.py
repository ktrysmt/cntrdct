#!/usr/bin/env python3
"""
Merge multiple SARIF 2.1.0 documents into a single document by
concatenating their `runs` arrays. The first document's top-level
`$schema` and `version` win.

Usage: merge_sarif.py <out_path> <file1> [<file2> ...]

SARIF natively supports multiple `runs` per document (one per tool /
invocation), so this is purely a JSON-shape merge — no normalisation
of rule IDs or thread-flow stitching is required for the cntrdct case
where every run came from the same tool with the same rules taxonomy.
"""

import json
import sys
from typing import List


def main(argv: List[str]) -> int:
    if len(argv) < 3:
        print("usage: merge_sarif.py <out_path> <file1> [<file2> ...]", file=sys.stderr)
        return 2
    out_path = argv[1]
    docs = []
    for path in argv[2:]:
        with open(path) as f:
            docs.append(json.load(f))

    if not docs:
        print("merge_sarif.py: no inputs", file=sys.stderr)
        return 2

    base = {
        "$schema": docs[0].get("$schema"),
        "version": docs[0].get("version"),
        "runs": [],
    }
    for doc in docs:
        runs = doc.get("runs", [])
        if not isinstance(runs, list):
            print(f"merge_sarif.py: malformed runs[] in input", file=sys.stderr)
            return 2
        base["runs"].extend(runs)

    with open(out_path, "w") as f:
        json.dump(base, f, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
