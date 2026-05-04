"""Synthetic fixture: 'may raise' phrasing without a raise statement."""


def authorize(user):
    """May raise PermissionError if the user is not active."""
    return user.id
