"""Negative fixture: the terminator lives inside an inner `if` block, so
the follower in the outer block is reachable when the condition is
false. Mirrors Rust T7."""


def maybe_emit(payload):
    if payload is None:
        return
    emit(payload)


def emit(payload):
    pass
