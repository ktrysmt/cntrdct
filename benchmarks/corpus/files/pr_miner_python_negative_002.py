"""pr-miner negative: every function correctly pairs begin_session/end_session."""


def safe_write_one(x):
    begin_session()
    r = x * 3
    end_session()
    return r


def safe_write_two():
    begin_session()
    r = True
    end_session()
    return r


def safe_write_three(n):
    begin_session()
    r = n + 1
    end_session()
    return r


def safe_write_four(flag):
    begin_session()
    _ = not flag
    end_session()


def safe_write_five(value):
    begin_session()
    r = value & 0x0f
    end_session()
    return r
