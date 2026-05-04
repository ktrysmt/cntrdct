"""Synthetic fixture: docstring promises a raise but body never raises."""


def parse_header(buf):
    """Raises ValueError when the header is shorter than four bytes."""
    return buf[:4]
