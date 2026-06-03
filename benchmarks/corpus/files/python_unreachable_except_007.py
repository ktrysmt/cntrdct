class MyError(ValueError):
    pass


def validate(value):
    try:
        return check(value)
    except Exception:
        return None
    except MyError:
        return False
