"""Synthetic fixture: file-fetch family using context manager, with one drifted variant."""
# Source: https://github.com/ktrysmt/cntrdct/blob/master/benchmarks/corpus/files/clone_drift_python_005.py

def fetch_a(path):
    with open(path) as f:
        data = f.read()
    return data


def fetch_b(path):
    with open(path) as f:
        data = f.read()
    return data


def fetch_c(path):
    with open(path) as f:
        data = f.read()
    return data


def fetch_d(path):
    with open(path) as f:
        data = f.read()
    return data


def fetch_drifted(path):
    with open(path) as f:
        data = f.read()
        log_access(path)
    return data
