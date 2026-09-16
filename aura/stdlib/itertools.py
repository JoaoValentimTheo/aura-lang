"""Aura Standard Library - Itertools module."""

def range_iter(start, end=None, step=1):
    """Create range iterator."""
    if end is None:
        end = start
        start = 0
    return range(start, end, step)

def cycle(iterable):
    """Cycle through iterable indefinitely."""
    from itertools import cycle as cycle_iter
    return cycle_iter(iterable)

def repeat(value, times=None):
    """Repeat value."""
    from itertools import repeat as repeat_iter
    return repeat_iter(value, times)

def chain(*iterables):
    """Chain iterables together."""
    from itertools import chain as chain_iter
    return chain_iter(*iterables)

def combinations(iterable, r):
    """Generate combinations."""
    from itertools import combinations as comb_iter
    return list(comb_iter(iterable, r))

def permutations(iterable, r=None):
    """Generate permutations."""
    from itertools import permutations as perm_iter
    if r is None:
        return list(perm_iter(iterable))
    return list(perm_iter(iterable, r))

def product(*iterables, repeat=1):
    """Cartesian product."""
    from itertools import product as prod_iter
    return list(prod_iter(*iterables, repeat=repeat))

def count(start=0, step=1):
    """Infinite counter."""
    from itertools import count as count_iter
    return count_iter(start, step)

def enumerate_iter(iterable, start=0):
    """Enumerate iterable."""
    return list(enumerate(iterable, start))

def islice(iterable, *args):
    """Slice iterator."""
    from itertools import islice as slice_iter
    return list(slice_iter(iterable, *args))

def takewhile(predicate, iterable):
    """Take while predicate is true."""
    from itertools import takewhile as take_iter
    return list(take_iter(predicate, iterable))

def dropwhile(predicate, iterable):
    """Drop while predicate is true."""
    from itertools import dropwhile as drop_iter
    return list(drop_iter(predicate, iterable))

def groupby(iterable, key=None):
    """Group items by key."""
    from itertools import groupby as group_iter
    return {k: list(g) for k, g in group_iter(iterable, key)}

def filterfalse(predicate, iterable):
    """Filter to keep items where predicate is false."""
    from itertools import filterfalse as ff
    return list(ff(predicate, iterable))

def starmap(fn, iterable):
    """Apply function to argument tuples."""
    from itertools import starmap as smap
    return list(smap(fn, iterable))

def tee(iterable, n=2):
    """Create n independent iterators."""
    from itertools import tee as tee_iter
    return list(tee_iter(iterable, n))

def zip_longest(*iterables, fillvalue=None):
    """Zip iterables with fill value."""
    from itertools import zip_longest as zl
    return list(zl(*iterables, fillvalue=fillvalue))

def pairwise(iterable):
    """Yield consecutive pairs."""
    from itertools import pairwise as pair
    return list(pair(iterable))
