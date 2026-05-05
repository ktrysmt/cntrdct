"""Synthetic fixture: list-validation family with one drifted variant."""


def validate_a(items):
    out = []
    for it in items:
        if it > 0:
            out.append(it)
    return out


def validate_b(items):
    out = []
    for it in items:
        if it > 0:
            out.append(it)
    return out


def validate_c(items):
    out = []
    for it in items:
        if it > 0:
            out.append(it)
    return out


def validate_d(items):
    out = []
    for it in items:
        if it > 0:
            out.append(it)
    return out


def validate_drifted(items):
    out = []
    for it in items:
        if it > 0 and it < 100:
            out.append(it)
    return out
