"""Synthetic fixture: file-fetch family using context manager, with one drifted variant."""


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
