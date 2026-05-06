"""pr-miner positive: begin_session/end_session pairing violated by leaked_session."""


def write_one(x):
    begin_session()
    r = x + 2
    end_session()
    return r


def write_two(a, b):
    begin_session()
    r = a // max(b, 1)
    end_session()
    return r


def write_three():
    begin_session()
    r = False
    end_session()
    return r


def write_four(n):
    begin_session()
    r = n + 7
    end_session()
    return r


def write_five(flag):
    begin_session()
    _ = bool(flag)
    end_session()


def write_six(value):
    begin_session()
    r = abs(value)
    end_session()
    return r


def write_seven():
    begin_session()
    _ = "x"
    end_session()


def leaked_session():
    begin_session()
    pr_miner_py_002_helper()
