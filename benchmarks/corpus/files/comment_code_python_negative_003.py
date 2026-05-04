"""Negative fixture: ordinary function with neutral docstring."""


def configure(payload):
    """Apply payload defaults and return the resulting record."""
    record = dict(payload)
    record.setdefault("retries", 3)
    return record
