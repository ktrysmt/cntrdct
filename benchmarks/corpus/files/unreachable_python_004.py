"""Synthetic fixture: `os._exit` followed by a finalizer that never runs.

`os._exit` skips Python finalizers, so the trailing `notify_parent`
call is unreachable in addition to being control-flow dead.
"""

import os


def fork_child():
    pid = 0
    if pid == 0:
        os._exit(0)
        notify_parent()
    return pid


def notify_parent():
    pass
