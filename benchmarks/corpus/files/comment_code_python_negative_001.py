"""Negative fixture: docstring matches body — promised raise is delivered."""


def parse_header(buf):
    """Raises ValueError when the header is shorter than four bytes."""
    if len(buf) < 4:
        raise ValueError("header truncated")
    return buf[:4]
