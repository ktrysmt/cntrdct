"""Negative fixture: target/source called in matching order."""


def move_bytes(target, source):
    return target + source


def driver():
    target = 1
    source = 2
    _ = move_bytes(target, source)
