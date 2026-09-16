"""Aura Standard Library - Math module."""

import builtins as _builtins
import math
from functools import reduce as _reduce

# Constants
PI = math.pi
E = math.e
TAU = math.tau
INF = math.inf
NAN = math.nan
# Lowercase aliases
pi = PI
e = E
tau = TAU
inf = INF
nan = NAN

# Basic functions
def abs(x):
    """Absolute value."""
    return _builtins.abs(x)

def min(*values):
    """Minimum value."""
    return _builtins.min(*values)

def max(*values):
    """Maximum value."""
    return _builtins.max(*values)

def round(x, ndigits=0):
    """Round number."""
    return _builtins.round(x, ndigits)

def floor(x):
    """Floor value."""
    return math.floor(x)

def ceil(x):
    """Ceiling value."""
    return math.ceil(x)

def sqrt(x):
    """Square root."""
    return math.sqrt(x)

def pow(x, y):
    """Power."""
    return math.pow(x, y)

def log(x, base=math.e):
    """Logarithm."""
    return math.log(x, base)

def log10(x):
    """Base-10 logarithm."""
    return math.log10(x)

def log2(x):
    """Base-2 logarithm."""
    return math.log2(x)

def exp(x):
    """Exponential (e^x)."""
    return math.exp(x)

def sin(x):
    """Sine (radians)."""
    return math.sin(x)

def cos(x):
    """Cosine (radians)."""
    return math.cos(x)

def tan(x):
    """Tangent (radians)."""
    return math.tan(x)

def asin(x):
    """Arc sine."""
    return math.asin(x)

def acos(x):
    """Arc cosine."""
    return math.acos(x)

def atan(x):
    """Arc tangent."""
    return math.atan(x)

def atan2(y, x):
    """Arc tangent (two arguments)."""
    return math.atan2(y, x)

def sinh(x):
    """Hyperbolic sine."""
    return math.sinh(x)

def cosh(x):
    """Hyperbolic cosine."""
    return math.cosh(x)

def tanh(x):
    """Hyperbolic tangent."""
    return math.tanh(x)

def degrees(x):
    """Convert radians to degrees."""
    return math.degrees(x)

def radians(x):
    """Convert degrees to radians."""
    return math.radians(x)

def gcd(*numbers):
    """Greatest common divisor. With no arguments returns 0."""
    if not numbers:
        return 0
    from math import gcd as gcd_fn
    return _reduce(gcd_fn, numbers)

def lcm(*numbers):
    """Least common multiple. With no arguments returns 1."""
    if not numbers:
        return 1
    from math import gcd as gcd_fn, lcm as lcm_fn
    return _reduce(lcm_fn, numbers)

def factorial(n):
    """Factorial."""
    return math.factorial(n)

def comb(n, k):
    """Combinations."""
    return math.comb(n, k)

def perm(n, k=None):
    """Permutations."""
    return math.perm(n, k)

def is_finite(x):
    """Check if number is finite."""
    return math.isfinite(x)

def is_infinite(x):
    """Check if number is infinite."""
    return math.isinf(x)

def is_nan(x):
    """Check if number is NaN."""
    return math.isnan(x)

def copysign(x, y):
    """Return x with sign of y."""
    return math.copysign(x, y)

def fabs(x):
    """Absolute value (float)."""
    return math.fabs(x)

def fmod(x, y):
    """Modulo (float)."""
    return math.fmod(x, y)

def fsum(iterable):
    """Accurate sum of floats."""
    return math.fsum(iterable)

def prod(iterable, start=1):
    """Product of values."""
    return math.prod(iterable, start=start)

def remainder(x, y):
    """IEEE remainder."""
    return math.remainder(x, y)

def dist(p1, p2):
    """Euclidean distance."""
    return math.dist(p1, p2)

def hypot(*coords):
    """Euclidean distance of origin."""
    return math.hypot(*coords)
