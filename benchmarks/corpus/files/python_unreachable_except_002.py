def fetch(key, table):
    try:
        return table[key]
    except LookupError:
        return None
    except KeyError:
        return "missing"
