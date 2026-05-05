"""Synthetic fixture: even split between two partitions of size 2. No drift expected."""


def shape_a_first(xs):
    out = []
    for x in xs:
        if x > 0:
            out.append(x)
    return out


def shape_a_second(xs):
    out = []
    for x in xs:
        if x > 0:
            out.append(x)
    return out


def shape_b_first(xs):
    out = []
    for x in xs:
        if x > 0:
            out.append(x)
        else:
            out.append(0)
    return out


def shape_b_second(xs):
    out = []
    for x in xs:
        if x > 0:
            out.append(x)
        else:
            out.append(0)
    return out
