"""Synthetic fixture: `assert False` followed by a comment-driven branch.

The author left `assert False` as a placeholder during a refactor and
never deleted the trailing fallback path — but `assert False` raises
unconditionally, so the fallback is unreachable.
"""


def reload_table(name):
    if name == "":
        assert False
        emit_warning(name)
    return name


def emit_warning(name):
    pass
