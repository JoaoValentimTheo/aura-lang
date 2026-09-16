"""Aura Standard Library initialization."""

__version__ = "0.1.0a3"
__author__ = "Aura Team"

# Core modules
from . import collections
from . import itertools
from . import math
from . import string
from . import json
from . import time
from . import io
from . import regex
from . import os
from . import http
from . import python

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
]

# Convenience imports
from .collections import (
    list_map,
    list_filter,
    list_reduce,
    dict_get,
    dict_keys,
    dict_values,
    set_union,
    set_intersection,
)

from .itertools import (
    range_iter,
    chain,
    combinations,
    permutations,
    enumerate_iter,
)

from .math import (
    PI, E, TAU,
    sqrt, pow, exp,
    sin, cos, tan,
    log, log10,
)

from .string import (
    upper, lower, trim,
    split, join, replace,
    starts_with, ends_with,
)

from .json import (
    loads, dumps, load, dump, pretty,
    parse, stringify, is_valid, merge,
)

from .time import (
    now, now_ms, sleep, clock,
    monotonic, perf_counter, strftime,
    iso, elapsed,
)

from .io import (
    read, write, append, exists,
    is_file, is_dir, mkdir, ls, rm,
    rename, basename, dirname, join,
    read_lines, write_lines, copy, size, touch,
)
