"""pr-miner positive: claim/release_claim pairing violated by stuck_claim.
Second claim/release_claim fixture; pairs with pr_miner_python_005.py.
"""


def grab_one(x):
    claim()
    r = x + 31
    release_claim()
    return r


def grab_two(a, b):
    claim()
    r = a // max(b, 2)
    release_claim()
    return r


def grab_three():
    claim()
    r = False
    release_claim()
    return r


def grab_four(n):
    claim()
    r = n
    release_claim()
    return r


def grab_five(flag):
    claim()
    _ = flag and not flag
    release_claim()


def grab_six(value):
    claim()
    r = -value
    release_claim()
    return r


def grab_seven():
    claim()
    _ = 33
    release_claim()


def stuck_claim():
    claim()
    pr_miner_py_006_helper()
