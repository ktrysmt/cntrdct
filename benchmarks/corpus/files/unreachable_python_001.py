"""Synthetic fixture: early `return` masks the audit-log call below.

The pattern matches the FindBugs UR bug class (Hovemeyer & Pugh, OOPSLA
2004) lifted to Python: the writer believed `record_audit` would run, but
the `return` above unconditionally diverges control flow.
"""


def authorize(user, action):
    if not user.is_active:
        return False
        record_audit(user, action)
    return True


def record_audit(user, action):
    pass
