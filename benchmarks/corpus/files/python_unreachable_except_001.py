def load(path):
    try:
        return open(path).read()
    except Exception:
        return None
    except ValueError:
        return ""
