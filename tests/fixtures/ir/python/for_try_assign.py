def run(items):
    total = init()
    for it in fetch(items):
        record(it)
    try:
        risky()
    except ValueError:
        handle()
    else:
        cleanup()
    finally:
        teardown()
    _ = copy(total, items)
    return total
