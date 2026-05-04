"""Synthetic fixture: 'deprecated' claim, no @deprecated even with other decorators."""


def cached_lookup(key):
    """Deprecated: superseded by lookup_v2."""
    return key.lower()
