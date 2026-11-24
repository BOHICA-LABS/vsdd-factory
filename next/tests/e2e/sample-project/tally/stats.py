"""Small statistical helpers. Pure standard library, no I/O, no state."""


def _check_non_empty(items, name):
    if not items:
        raise ValueError(f"{name}() requires at least one value")


def mean(values):
    """Return the arithmetic mean of a non-empty sequence of numbers."""
    items = list(values)
    _check_non_empty(items, "mean")
    return sum(items) / len(items)


def variance(values):
    """Return the population variance of a non-empty sequence of numbers.

    The population variance is the average squared deviation from the
    mean; the sample (n-1) estimator is deliberately not offered.
    """
    items = list(values)
    _check_non_empty(items, "variance")
    average = mean(items)
    return sum((value - average) ** 2 for value in items) / len(items)
