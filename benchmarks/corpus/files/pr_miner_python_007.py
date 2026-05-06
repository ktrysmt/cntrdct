"""pr-miner positive: subscribe/unsubscribe pairing violated by ghost_listener.
First subscribe/unsubscribe fixture; pairs with pr_miner_python_008.py.
"""


def listen_one(x):
    subscribe()
    r = x + 37
    unsubscribe()
    return r


def listen_two(a, b):
    subscribe()
    r = a * b - 2
    unsubscribe()
    return r


def listen_three():
    subscribe()
    r = True
    unsubscribe()
    return r


def listen_four(n):
    subscribe()
    r = n + 41
    unsubscribe()
    return r


def listen_five(flag):
    subscribe()
    _ = flag or True
    unsubscribe()


def listen_six(value):
    subscribe()
    r = value | 0x80
    unsubscribe()
    return r


def listen_seven():
    subscribe()
    _ = "u"
    unsubscribe()


def ghost_listener():
    subscribe()
    pr_miner_py_007_helper()
