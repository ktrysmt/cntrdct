"""Synthetic fixture: 'throws' phrasing without a raise statement."""


def divide(x, y):
    """Throws ZeroDivisionError when y is zero."""
    return x // (y or 1)
