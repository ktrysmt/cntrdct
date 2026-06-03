def read_config(path):
    try:
        with open(path) as fh:
            return fh.read()
    except OSError:
        return None
    except FileNotFoundError:
        return "default"
