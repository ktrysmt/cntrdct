# Source: https://github.com/carljm/django-secure/blob/cfad71fec00d28fdb2256930df4ce861a94a6056/doc/conf.py
# License: BSD-3-Clause
# Note: minimal extract from carljm/django-secure@cfad71fe doc/conf.py (upstream 225 lines, BSD-3-Clause). Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The upstream file is a Sphinx `conf.py` that contains one top-level def — `get_version()` at upstream line 49 — which opens `djangosecure/__init__.py` in a try/finally with `fh.close()` in finally. The remaining 200+ lines of the upstream file are module-level Sphinx configuration assignments and are excluded by pr-miner's spec F2 extractor (only `function_definition` and `decorated_definition` are walked). This minimal extract therefore keeps only the imports `get_version` needs (`from os.path import join, dirname`) plus the `get_version` def, so the file parses cleanly under tree-sitter Python 3 with no extraneous module-level state. The Semgrep `open-never-closed` rule produces no finding on `get_version` because the try/finally block guarantees `fh.close()` even on exception. pr-miner's spec F2 extracts the item set `{open, join, dirname, readlines, startswith, split, strip, close`}, contributing one paired open+close transaction to the mining database. The file's net pr-miner contribution is +1 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the extracted file as committed (per benchmarks/audit-corpus/README.md "minimal extracts" clause).

# -*- coding: utf-8 -*-

from os.path import join, dirname
def get_version():
    fh = open(join(dirname(dirname(__file__)), "djangosecure", "__init__.py"))
    try:
        for line in fh.readlines():
            if line.startswith("__version__ ="):
                return line.split("=")[1].strip().strip('"')
    finally:
        fh.close()

