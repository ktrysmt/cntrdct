"""Synthetic fixture: four exact clones (same normalized form). No drift expected."""


def alpha(xs):
    out = []
    for x in xs:
        out.append(x * 2)
    return out


def beta(items):
    result = []
    for elem in items:
        result.append(elem * 2)
    return result


def gamma(values):
    accum = []
    for v in values:
        accum.append(v * 2)
    return accum


def delta(arr):
    res = []
    for a in arr:
        res.append(a * 2)
    return res
