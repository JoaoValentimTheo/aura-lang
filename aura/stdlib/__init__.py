"""Aura Standard Library initialization."""

from aura import __version__

__author__ = "Aura Team"

# Core modules
from . import (
    asyncio,
    collections,
    crypto,
    crypto_backend,
    http,
    io,
    itertools,
    json,
    math,
    os,
    python,
    regex,
    string,
    testing,
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
    'testing',
    'crypto',
    'crypto_backend',
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
    ls,
    mkdir,
    read,
    read_async,
    read_lines,
    rename,
    rm,
    size,
    touch,
    write,
    write_async,
    write_lines,
)
from .io import (
    join as path_join,
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
    INF,
    NAN,
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
