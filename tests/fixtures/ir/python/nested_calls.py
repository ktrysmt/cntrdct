def outer(a, b):
    return inner(a, plus(b, 1))


def inner(x, y):
    return x + y


def plus(x, y):
    return x + y
