def decode(buf):
    try:
        return buf.decode("utf-8")
    except ValueError:
        return None
    except UnicodeDecodeError:
        return b""
