"""Negative fixture: ternary callee — out of v0 scope (n != 2)."""


def three(a, b, c):
    return a + b + c


def driver():
    a = 1
    b = 2
    c = 3
    _ = three(c, b, a)
