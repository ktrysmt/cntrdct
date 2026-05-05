"""Negative fixture: keyword args make the binding explicit; v0 skips."""


def pair(dst, src):
    return dst + src


def driver():
    dst = 1
    src = 2
    _ = pair(src=src, dst=dst)
