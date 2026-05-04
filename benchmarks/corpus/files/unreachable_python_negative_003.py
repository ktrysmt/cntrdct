"""Negative fixture: ordinary straight-line code with no diverging
terminator. `assert True` is not a terminator. Mirrors Rust T13."""


def configure(payload):
    assert True
    name = payload.get("name")
    save(name)


def save(name):
    pass
