"""Synthetic fixture: `raise` followed by a stale cleanup call.

After the function began raising on invalid input, the trailing
`close_handle()` was never updated and is now unreachable.
"""


def parse_header(buf):
    if len(buf) < 4:
        raise ValueError("header truncated")
        close_handle(buf)
    return buf[:4]


def close_handle(buf):
    pass
