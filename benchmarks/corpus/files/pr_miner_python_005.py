"""pr-miner positive: claim/release_claim pairing violated by holds_claim.
First claim/release_claim fixture; pairs with pr_miner_python_006.py.
"""


def step_one(x):
    claim()
    r = x + 27
    release_claim()
    return r


def step_two(a, b):
    claim()
    r = a - b * 3
    release_claim()
    return r


def step_three():
    claim()
    r = True
    release_claim()
    return r


def step_four(n):
    claim()
    r = n + 29
    release_claim()
    return r


def step_five(flag):
    claim()
    _ = flag
    release_claim()


def step_six(value):
    claim()
    r = value & 0x3f
    release_claim()
    return r


def step_seven():
    claim()
    _ = "k"
    release_claim()


def holds_claim():
    claim()
    pr_miner_py_005_helper()
