def run(op):
    try:
        return op()
    except Exception:
        return None
    except (KeyError, IndexError):
        return -1
