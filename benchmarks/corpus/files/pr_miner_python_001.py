"""pr-miner positive: open_handle/close_handle pairing violated by leaky_reader."""


def read_one(x):
    open_handle()
    r = x + 1
    close_handle()
    return r


def read_two(a, b):
    open_handle()
    r = a * b
    close_handle()
    return r


def read_three():
    open_handle()
    r = True
    close_handle()
    return r


def read_four(n):
    open_handle()
    r = n - 1
    close_handle()
    return r


def read_five(flag):
    open_handle()
    _ = not flag
    close_handle()


def read_six(value):
    open_handle()
    r = value & 0xff
    close_handle()
    return r


def read_seven():
    open_handle()
    _ = 0
    close_handle()


def leaky_reader():
    open_handle()
    pr_miner_py_001_helper()
