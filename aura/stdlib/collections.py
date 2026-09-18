"""Aura Standard Library - Collections module."""
import builtins
import itertools as _itertools
from functools import reduce as _py_reduce

_MISSING = object()


class AuraDict(dict):
    """A dict that also supports attribute access (`user.name`)."""

    def __getattribute__(self, name):
        # A data key takes precedence over an inherited dict method so a
        # mapping that happens to contain `keys`, `values`, `items`, `get`,
        # ... is still reachable as `d.keys`. Dunder lookups are left alone so
        # copy/pickle/inspect keep working, and methods keep working when no
        # such key exists. `self.keys()` is a real call: if the stored value is
        # callable it is returned and invoked like any other member.
        if not (name.startswith('__') and name.endswith('__')):
            try:
                return dict.__getitem__(self, name)
            except KeyError:
                pass
        return dict.__getattribute__(self, name)

    def __getattr__(self, name):
        try:
            return self[name]
        except KeyError as exc:
            raise AttributeError(name) from exc

    def __setattr__(self, name, value):
        self[name] = value



def list_map(fn, items):
    """Map function over list."""
    return list(builtins.map(fn, items))

def list_filter(predicate, items):
    """Filter list by predicate."""
    return list(builtins.filter(predicate, items))

def list_reduce(fn, items, initial=_MISSING):
    """Reduce list to single value."""
    if initial is _MISSING:
        return _py_reduce(fn, items)
    return _py_reduce(fn, items, initial)

def list_find(predicate, items):
    """Find first item matching predicate."""
    for item in items:
        if predicate(item):
            return item
    return None

def list_any(predicate, items):
    """Check if any item matches predicate."""
    return any(predicate(item) for item in items)

def list_all(predicate, items):
    """Check if all items match predicate."""
    return all(predicate(item) for item in items)

def list_take(n, items):
    """Take first n items (works with any iterable)."""
    if n <= 0:
        return []
    return list(_itertools.islice(items, n))

def list_drop(n, items):
    """Drop first n items (works with any iterable)."""
    if n <= 0:
        return list(items)
    return list(_itertools.islice(items, n, None))

def list_zip(*iterables):
    """Zip lists together."""
    return list(zip(*iterables, strict=False))

def list_flatten(items):
    """Flatten nested list."""
    result = []
    for item in items:
        if isinstance(item, (list, tuple)):
            result.extend(list_flatten(item))
        else:
            result.append(item)
    return result

def list_unique(items):
    """Get unique items preserving order."""
    seen = set()
    result = []
    for item in items:
        if item not in seen:
            seen.add(item)
            result.append(item)
    return result

def list_sort(items, key=None, reverse=False):
    """Sort list."""
    return sorted(items, key=key, reverse=reverse)

def list_reverse(items):
    """Reverse list."""
    return list(reversed(items))

def list_chunk(n, items):
    """Split list into chunks of size n."""
    if n <= 0:
        raise ValueError("chunk size must be positive")
    items = list(items)
    return [items[i:i+n] for i in range(0, len(items), n)]


def dict_get(d, key, default=None):
    """Get dict value with default."""
    return d.get(key, default)

def dict_keys(d):
    """Get dict keys as list."""
    return list(d.keys())

def dict_values(d):
    """Get dict values as list."""
    return list(d.values())

def dict_items(d):
    """Get dict items as list."""
    return list(d.items())

def dict_merge(*dicts):
    """Merge multiple dicts."""
    result = {}
    for d in dicts:
        result.update(d)
    return result

def dict_filter(predicate, d):
    """Filter dict by predicate."""
    return {k: v for k, v in d.items() if predicate(k, v)}

def dict_map(fn, d):
    """Map function over dict values."""
    return {k: fn(v) for k, v in d.items()}


def set_union(*sets):
    """Union of sets."""
    if not sets:
        return set()
    return set().union(*sets)

def set_intersection(*sets):
    """Intersection of sets (a fresh set, never an input alias)."""
    if not sets:
        return set()
    result = set(sets[0])
    if len(sets) > 1:
        result.intersection_update(*sets[1:])
    return result

def set_difference(a, *rest):
    """Difference of sets (a fresh set, never an input alias)."""
    result = set(a)
    if rest:
        result.difference_update(*rest)
    return result

# Aliases for convenience (data-first, so `data |> map(fn)` works)
def map(items, fn):
    """Map function over items. Data-first for pipe compatibility."""
    return list(builtins.map(fn, items))

def filter(items, predicate):
    """Filter items by predicate. Data-first for pipe compatibility."""
    return list(builtins.filter(predicate, items))

def reduce(items, fn, initial=_MISSING):
    """Reduce items to a single value. Data-first for pipe compatibility."""
    if initial is _MISSING:
        return _py_reduce(fn, items)
    return _py_reduce(fn, items, initial)

def take(items, n):
    """Take first n items. Data-first for pipe compatibility."""
    if n <= 0:
        return []
    return list(_itertools.islice(items, n))

def drop(items, n):
    """Drop first n items. Data-first for pipe compatibility."""
    if n <= 0:
        return list(items)
    return list(_itertools.islice(items, n, None))
