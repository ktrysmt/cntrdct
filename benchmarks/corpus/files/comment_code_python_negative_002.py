"""Negative fixture: docstring marks function deprecated and decorator agrees."""

import warnings


@warnings.deprecated("use render_v2 instead")
def legacy_render(payload):
    """Deprecated: use render_v2 instead."""
    return str(payload)
