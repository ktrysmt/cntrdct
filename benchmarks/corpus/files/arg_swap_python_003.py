"""Synthetic fixture: default params on render(height, width) — swapped call."""


def render(width=10, height=20):
    return width * height


def view():
    width = 100
    height = 200
    _ = render(height, width)
