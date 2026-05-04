"""Synthetic fixture: docstring marks function deprecated but no decorator."""


def legacy_render(payload):
    """Deprecated: use render_v2 instead."""
    return str(payload)
