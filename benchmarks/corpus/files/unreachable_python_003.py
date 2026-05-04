"""Synthetic fixture: `sys.exit` followed by a stale shutdown step.

The author called `sys.exit(2)` for a fatal config error and forgot the
follow-up step is unreachable.
"""

import sys


def main(config):
    if not config.valid:
        sys.exit(2)
        flush_metrics(config)
    run(config)


def flush_metrics(config):
    pass


def run(config):
    pass
