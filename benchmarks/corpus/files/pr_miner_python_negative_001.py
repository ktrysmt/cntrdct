"""pr-miner negative: every function correctly pairs open_handle/close_handle."""


def safe_read_one(x):
    open_handle()
    r = x + 10
    close_handle()
    return r


def safe_read_two():
    open_handle()
    r = False
    close_handle()
    return r


def safe_read_three(n):
    open_handle()
    r = n
    close_handle()
    return r


def safe_read_four(flag):
    open_handle()
    _ = bool(flag)
    close_handle()


def safe_read_five(value):
    open_handle()
    r = value * 2
    close_handle()
    return r
