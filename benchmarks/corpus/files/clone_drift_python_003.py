"""Synthetic fixture: while-loop reduction family with one drifted variant."""


def reduce_a(xs):
    total = 0
    i = 0
    while i < len(xs):
        total += xs[i]
        i += 1
    return total


def reduce_b(xs):
    total = 0
    i = 0
    while i < len(xs):
        total += xs[i]
        i += 1
    return total


def reduce_c(xs):
    total = 0
    i = 0
    while i < len(xs):
        total += xs[i]
        i += 1
    return total


def reduce_d(xs):
    total = 0
    i = 0
    while i < len(xs):
        total += xs[i]
        i += 1
    return total


def reduce_drifted(xs):
    total = 0
    i = 0
    while i < len(xs):
        if xs[i] > 0:
            total += xs[i]
        i += 1
    return total
