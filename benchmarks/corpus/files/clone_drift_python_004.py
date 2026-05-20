"""Synthetic fixture: string-classifier family with one drifted variant."""
# Source: https://github.com/ktrysmt/cntrdct/blob/master/benchmarks/corpus/files/clone_drift_python_004.py

def classify_a(s):
    if s.startswith("get"):
        return "GET"
    elif s.startswith("post"):
        return "POST"
    else:
        return "UNKNOWN"


def classify_b(s):
    if s.startswith("get"):
        return "GET"
    elif s.startswith("post"):
        return "POST"
    else:
        return "UNKNOWN"


def classify_c(s):
    if s.startswith("get"):
        return "GET"
    elif s.startswith("post"):
        return "POST"
    else:
        return "UNKNOWN"


def classify_d(s):
    if s.startswith("get"):
        return "GET"
    elif s.startswith("post"):
        return "POST"
    else:
        return "UNKNOWN"


def classify_drifted(s):
    if s.startswith("get"):
        return "GET"
    elif s.startswith("post"):
        return "POST"
    else:
        return s.upper()
