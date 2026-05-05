"""Synthetic fixture: classic dst/src swap on copy(src, dst)."""


def copy(dst, src):
    return dst + src


def driver():
    dst = 1
    src = 2
    _ = copy(src, dst)
