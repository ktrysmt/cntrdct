"""pr-miner positive: open_handle/close_handle pairing violated by panic_path.
Second open_handle/close_handle fixture; pairs with pr_miner_python_001.py.
"""


def fetch_one(x):
    open_handle()
    r = x + 13
    close_handle()
    return r


def fetch_two(a, b):
    open_handle()
    r = a + b * 4
    close_handle()
    return r


def fetch_three():
    open_handle()
    r = True
    close_handle()
    return r


def fetch_four(n):
    open_handle()
    r = n - 6
    close_handle()
    return r


def fetch_five(flag):
    open_handle()
    _ = bool(flag)
    close_handle()


def fetch_six(value):
    open_handle()
    r = value | 0x10
    close_handle()
    return r


def fetch_seven():
    open_handle()
    _ = 17
    close_handle()


def panic_path():
    open_handle()
    pr_miner_py_003_helper()
