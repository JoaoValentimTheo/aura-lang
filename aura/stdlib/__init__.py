"""Aura Standard Library initialization."""

from aura import __version__

__author__ = "Aura Team"

# Core modules
from . import (
    asyncio,
    collections,
    http,
    io,
    itertools,
    json,
    math,
    os,
    python,
    regex,
    string,
    threading,
    time,
)

# Common exports
__all__ = [
    'collections',
    'itertools',
    'math',
    'string',
    'json',
    'time',
    'io',
    'regex',
    'os',
    'http',
    'python',
    'threading',
    'asyncio',
]

# Convenience imports
from .collections import (
    dict_get,
    dict_keys,
    dict_values,
    list_filter,
    list_map,
    list_reduce,
    set_intersection,
    set_union,
)
from .io import (
    append,
    basename,
    copy,
    dirname,
    exists,
    is_dir,
    is_file,
    join,
    ls,
    mkdir,
    read,
    read_lines,
    rename,
    rm,
    size,
    touch,
    write,
    write_lines,
)
from .itertools import (
    chain,
    combinations,
    enumerate_iter,
    permutations,
    range_iter,
)
from .json import (
    dump,
    dumps,
    is_valid,
    load,
    loads,
    merge,
    parse,
    pretty,
    stringify,
)
from .math import (
    PI,
    TAU,
    E,
    cos,
    exp,
    log,
    log10,
    pow,
    sin,
    sqrt,
    tan,
)
from .string import (
    ends_with,
    join,
    lower,
    replace,
    split,
    starts_with,
    trim,
    upper,
)
from .time import (
    clock,
    elapsed,
    iso,
    monotonic,
    now,
    now_ms,
    perf_counter,
    sleep,
    strftime,
)
