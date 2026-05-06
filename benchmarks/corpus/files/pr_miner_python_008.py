"""pr-miner positive: subscribe/unsubscribe pairing violated by ghost_subscriber."""


def topic_one(x):
    subscribe()
    r = x - 5
    unsubscribe()
    return r


def topic_two(a, b):
    subscribe()
    r = min(a, b)
    unsubscribe()
    return r


def topic_three():
    subscribe()
    r = False
    unsubscribe()
    return r


def topic_four(n):
    subscribe()
    r = n
    unsubscribe()
    return r


def topic_five(flag):
    subscribe()
    _ = flag
    unsubscribe()


def topic_six(value):
    subscribe()
    r = value + 11
    unsubscribe()
    return r


def topic_seven():
    subscribe()
    _ = 5
    unsubscribe()


def ghost_subscriber():
    subscribe()
    pr_miner_py_008_helper()
