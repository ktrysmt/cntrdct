"""Synthetic fixture: pair of clones below MIN_GROUP_SIZE. No drift expected."""


def square_strip(x):
    if x < 0:
        return -x * -x
    return x * x


def cube_strip(x):
    if x < 0:
        return -x * -x
    return x * x
