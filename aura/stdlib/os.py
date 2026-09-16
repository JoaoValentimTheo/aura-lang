"""Aura Standard Library - Operating system module.

Environment variables, paths, working directory and process information.
Process execution is intentionally *not* exposed here; use Python's
``subprocess`` explicitly when needed, so Aura never silently shells out.
"""

import os as _os
import sys as _sys
from pathlib import Path as _Path


def get_env(name, default=None):
    """Read an environment variable, with an optional default."""
    return _os.environ.get(name, default)


def set_env(name, value):
    """Set an environment variable for the current process."""
    _os.environ[name] = str(value)


def unset_env(name):
    """Remove an environment variable."""
    _os.environ.pop(name, None)


def env():
    """Return all environment variables as a dict."""
    return dict(_os.environ)


def cwd():
    """Current working directory."""
    return _os.getcwd()


def chdir(path):
    """Change the current working directory."""
    _os.chdir(path)


def home():
    """User's home directory."""
    return str(_Path.home())


def temp_dir():
    """System temporary directory."""
    import tempfile
    return tempfile.gettempdir()


def path_join(*parts):
    """Join path components using the OS separator."""
    return _os.path.join(*parts)


def path_abspath(path):
    """Absolute, normalized version of ``path``."""
    return _os.path.abspath(path)


def path_exists(path):
    """True when ``path`` exists."""
    return _os.path.exists(path)


def path_is_file(path):
    """True when ``path`` is a regular file."""
    return _os.path.isfile(path)


def path_is_dir(path):
    """True when ``path`` is a directory."""
    return _os.path.isdir(path)


def path_basename(path):
    """Final component of ``path``."""
    return _os.path.basename(path)


def path_dirname(path):
    """Directory component of ``path``."""
    return _os.path.dirname(path)


def path_split(path):
    """Split ``path`` into (head, tail)."""
    return list(_os.path.split(path))


def path_splitext(path):
    """Split ``path`` into (root, extension)."""
    return list(_os.path.splitext(path))


def path_normpath(path):
    """Normalize ``path`` (collapse ``..`` and ``.``)."""
    return _os.path.normpath(path)


def sep():
    """Path separator for the current OS."""
    return _os.sep


def linesep():
    """Line separator for the current OS."""
    return _os.linesep


def name():
    """OS name (``posix``, ``nt``, ...)."""
    return _os.name


def platform():
    """Platform identifier."""
    return _sys.platform


def pid():
    """Current process id."""
    return _os.getpid()


def listdir(path="."):
    """List directory entries (unsorted, like the OS)."""
    return _os.listdir(path)


def walk(path="."):
    """Walk a directory tree, returning (root, dirs, files) tuples."""
    return [list(item) for item in _os.walk(path)]


def makedirs(path, exist_ok=True):
    """Create a directory and its parents."""
    _os.makedirs(path, exist_ok=exist_ok)


def remove(path):
    """Remove a file."""
    _os.remove(path)


def rename(old, new):
    """Rename or move a file/directory."""
    _os.rename(old, new)


def get_size(path):
    """Size of a file in bytes."""
    return _os.path.getsize(path)