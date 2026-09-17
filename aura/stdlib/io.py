"""Aura Standard Library - File I/O module.

The synchronous helpers are the primary API. Each has an ``*_async`` sibling
that runs the same operation on a worker thread via ``asyncio.to_thread``, so
Aura's ``async def`` code can do file I/O without blocking the event loop. The
async variants are thin wrappers: they perform identically and raise the same
errors, differing only in how they are awaited.
"""

import asyncio as _asyncio
import shutil as _shutil
from pathlib import Path as _Path

_ENCODING = 'utf-8'


def read(path):
    """Read entire file contents as a UTF-8 string."""
    return _Path(path).read_text(encoding=_ENCODING)


def write(path, content):
    """Write string content to file (creates/overwrites)."""
    _Path(path).write_text(content, encoding=_ENCODING)


def append(path, content):
    """Append string content to file."""
    with open(path, 'a', encoding=_ENCODING) as f:
        f.write(content)


def exists(path):
    """Check if file or directory exists."""
    return _Path(path).exists()


def is_file(path):
    """Check if path is a file."""
    return _Path(path).is_file()


def is_dir(path):
    """Check if path is a directory."""
    return _Path(path).is_dir()


def mkdir(path):
    """Create directory (including parents)."""
    _Path(path).mkdir(parents=True, exist_ok=True)


def ls(path="."):
    """List directory contents (names only, sorted for determinism)."""
    return sorted(entry.name for entry in _Path(path).iterdir())


def rm(path):
    """Remove a file. Missing files raise an error (use exists() to guard)."""
    _Path(path).unlink()


def rename(old, new):
    """Rename/move a file."""
    _Path(old).rename(new)


def extension(path):
    """Get file extension (without dot)."""
    return _Path(path).suffix.lstrip('.')


def basename(path):
    """Get filename without directory."""
    return _Path(path).name


def dirname(path):
    """Get directory part of path."""
    return str(_Path(path).parent)


def join(*parts):
    """Join path components."""
    return str(_Path(*parts))


def read_lines(path):
    """Read file as a list of lines (newlines removed, blank lines kept)."""
    text = _Path(path).read_text(encoding=_ENCODING)
    return text.splitlines()


def write_lines(path, lines):
    """Write an iterable of lines to file (adds a trailing newline).

    An empty iterable writes an empty file rather than a lone newline.
    """
    materialized = [str(line) for line in lines]
    text = '\n'.join(materialized)
    if materialized:
        text += '\n'
    _Path(path).write_text(text, encoding=_ENCODING)


def copy(src, dst):
    """Copy a file."""
    _shutil.copy2(src, dst)


def size(path):
    """Get file size in bytes."""
    return _Path(path).stat().st_size


def touch(path):
    """Create empty file or update timestamp."""
    _Path(path).touch()


# ============================================================================
# Async variants
# ============================================================================
#
# These await the synchronous implementation on a worker thread. That keeps the
# event loop responsive for the common case (reading/writing a file) without
# introducing a second, subtly different implementation that could drift.

async def read_async(path):
    """Await :func:`read` off the event loop."""
    return await _asyncio.to_thread(read, path)


async def write_async(path, content):
    """Await :func:`write` off the event loop."""
    return await _asyncio.to_thread(write, path, content)


async def append_async(path, content):
    """Await :func:`append` off the event loop."""
    return await _asyncio.to_thread(append, path, content)


async def exists_async(path):
    """Await :func:`exists` off the event loop."""
    return await _asyncio.to_thread(exists, path)


async def is_file_async(path):
    """Await :func:`is_file` off the event loop."""
    return await _asyncio.to_thread(is_file, path)


async def is_dir_async(path):
    """Await :func:`is_dir` off the event loop."""
    return await _asyncio.to_thread(is_dir, path)


async def mkdir_async(path):
    """Await :func:`mkdir` off the event loop."""
    return await _asyncio.to_thread(mkdir, path)


async def ls_async(path="."):
    """Await :func:`ls` off the event loop."""
    return await _asyncio.to_thread(ls, path)


async def rm_async(path):
    """Await :func:`rm` off the event loop."""
    return await _asyncio.to_thread(rm, path)


async def rename_async(old, new):
    """Await :func:`rename` off the event loop."""
    return await _asyncio.to_thread(rename, old, new)


async def read_lines_async(path):
    """Await :func:`read_lines` off the event loop."""
    return await _asyncio.to_thread(read_lines, path)


async def write_lines_async(path, lines):
    """Await :func:`write_lines` off the event loop."""
    return await _asyncio.to_thread(write_lines, path, lines)


async def copy_async(src, dst):
    """Await :func:`copy` off the event loop."""
    return await _asyncio.to_thread(copy, src, dst)


async def size_async(path):
    """Await :func:`size` off the event loop."""
    return await _asyncio.to_thread(size, path)


async def touch_async(path):
    """Await :func:`touch` off the event loop."""
    return await _asyncio.to_thread(touch, path)
