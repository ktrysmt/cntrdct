"""pr-miner positive: begin_session/end_session pairing violated by abort_session.
Second begin_session/end_session fixture; pairs with pr_miner_python_002.py.
"""


def session_one(x):
    begin_session()
    r = x * 4
    end_session()
    return r


def session_two(a, b):
    begin_session()
    r = max(a, b) + 19
    end_session()
    return r


def session_three():
    begin_session()
    r = False
    end_session()
    return r


def session_four(n):
    begin_session()
    r = n + 23
    end_session()
    return r


def session_five(flag):
    begin_session()
    _ = not flag
    end_session()


def session_six(value):
    begin_session()
    r = value % 13
    end_session()
    return r


def session_seven():
    begin_session()
    _ = "z"
    end_session()


def abort_session():
    begin_session()
    pr_miner_py_004_helper()
