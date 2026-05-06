"""pr-miner negative: each function correctly pairs its own API call.
Mixes pairs from several positive scenarios so the corpus shows clean
negatives spread across rules.
"""


def safe_capture(x):
    start_recording()
    r = x + 4
    stop_recording()
    return r


def safe_mount(n):
    mount()
    r = n
    unmount()
    return r


def safe_fixture():
    setup_resource()
    r = True
    teardown_resource()
    return r


def safe_block(value):
    enter_block()
    r = value + 1
    exit_block()
    return r


def safe_claim(flag):
    claim()
    _ = not flag
    release_claim()


def safe_subscribe():
    subscribe()
    _ = 0
    unsubscribe()
