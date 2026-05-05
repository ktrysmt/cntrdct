"""Synthetic fixture: @lru_cache-wrapped cached_copy(src, dst) — swapped call."""

from functools import lru_cache


@lru_cache
def cached_copy(dst, src):
    return dst + src


def driver():
    dst = 1
    src = 2
    _ = cached_copy(src, dst)
