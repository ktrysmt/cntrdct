#!/usr/bin/env python3
"""
Read a cntrdct JSON findings array from stdin and emit one GitHub
workflow command (`::warning::` / `::error::` / `::notice::`) per
finding, formatted so the annotation appears inline on the PR diff.

Used by the cntrdct GitHub Action when output: annotations.
"""

import json
import sys

SEVERITY_TO_LEVEL = {
    "Error": "error",
    "Warning": "warning",
    "Note": "notice",
    "Info": "notice",
}


def main() -> int:
    data = json.load(sys.stdin)
    for rf in data:
        f = rf["finding"]
        level = SEVERITY_TO_LEVEL.get(f.get("raw_severity", "Warning"), "warning")
        primary = f["primary"]
        detector = f.get("detector_id", "?")
        message = f.get("message", "").replace("\n", " ").replace("\r", " ").replace("::", ":")
        print(
            "::{level} file={file},line={line},col={col},endLine={el},endColumn={ec},"
            "title=cntrdct::{detector}::[{detector}] {message}".format(
                level=level,
                file=primary["file"],
                line=primary["start_line"],
                col=primary["start_col"],
                el=primary["end_line"],
                ec=primary["end_col"],
                detector=detector,
                message=message,
            )
        )
    print(f"::notice::cntrdct produced {len(data)} findings", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
