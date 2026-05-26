class Counter:
    """A small class with two methods."""

    def __init__(self, start):
        """Construct a new counter."""
        self.value = start

    def add(self, delta):
        self.value = self.value + delta
