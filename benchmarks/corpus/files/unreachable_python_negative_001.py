"""Negative fixture: terminator is the last statement in its block, so
no follower is unreachable. Mirrors Rust T2."""


def early_return_only(value):
    if value is None:
        return
    process(value)


def process(value):
    pass
