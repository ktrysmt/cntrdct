"""Synthetic fixture: typed params on apply(source, target) — swapped call."""


def apply(target: int, source: int) -> int:
    return target + source


def caller():
    target = 1
    source = 2
    _ = apply(source, target)
