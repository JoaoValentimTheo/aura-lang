"""Aura Standard Library - Cryptography module.

This module has two layers:

1. **Classical primitives** built on Python's standard library, which are real
   and safe to use: hashing (SHA-2/SHA-3/SHAKE), HMAC, HKDF, and cryptographically
   secure random bytes. Prefer `sha3_256` / `sha3_512` / `shake_256` when you
   want post-quantum-resistant hashing.

2. **Post-quantum key encapsulation and signatures** (ML-KEM/Kyber,
   ML-DSA/Dilithium, SLH-DSA/SPHINCS+) through a *pluggable backend*.

Backend selection
-----------------

* If the optional `cryptography` package provides the algorithm (FIPS 203/204/
  205), it is used. Install it with ``pip install "aura-language[pqc]"``.
* Otherwise a bundled **pure-Python reference backend** is used. It has the
  correct API shapes and round-trips, but it is a *demonstration*, not a
  validated implementation. Do **not** protect real secrets with it.

Call `backend_info()` to see which backend is active. `require_production_backend()`
turns the fallback into a hard error when you need a guarantee.

Example::

    import stdlib.crypto as crypto

    print(crypto.backend_info())

    let digest = crypto.sha3_256("hello")
    let key = crypto.random_bytes(32)
    let mac = crypto.hmac_sha256(key, "message")

    let kp = crypto.kem_keypair()
    let enc = crypto.kem_encapsulate(kp.public_key)
    let shared = crypto.kem_decapsulate(kp.secret_key, enc.ciphertext)
    print(shared == enc.shared_secret)
"""

import hashlib as _hashlib
import hmac as _hmac
import secrets as _secrets

from . import crypto_backend as _backend
from .collections import AuraDict

# ============================================================================
# Backend information
# ============================================================================

def backend_info():
    """Return a dict describing the active post-quantum backend.

    Keys: ``name`` (``"cryptography"`` or ``"reference"``), ``production``
    (bool), and ``algorithms`` (list of supported PQC algorithm names).
    """
    return AuraDict(_backend.info())


def is_production_backend() -> bool:
    """True when the active backend is a vetted implementation."""
    return bool(_backend.info()["production"])


def require_production_backend():
    """Raise unless a vetted backend is active.

    Use this before relying on PQC for real secrets, so a missing dependency
    fails loudly instead of silently using the reference backend.
    """
    if not is_production_backend():
        raise RuntimeError(
            "no production post-quantum backend is installed; "
            'install aura-language[pqc] (cryptography>=44) for ML-KEM/ML-DSA'
        )


# ============================================================================
# Hashing (real, post-quantum-resistant digests)
# ============================================================================

def sha256(data) -> str:
    """SHA-256 hex digest of ``data`` (str or bytes)."""
    return _hashlib.sha256(_to_bytes(data)).hexdigest()


def sha512(data) -> str:
    """SHA-512 hex digest."""
    return _hashlib.sha512(_to_bytes(data)).hexdigest()


def sha3_256(data) -> str:
    """SHA3-256 hex digest (post-quantum-resistant hash)."""
    return _hashlib.sha3_256(_to_bytes(data)).hexdigest()


def sha3_512(data) -> str:
    """SHA3-512 hex digest."""
    return _hashlib.sha3_512(_to_bytes(data)).hexdigest()


def shake_256(data, length=32) -> bytes:
    """SHAKE256 XOF output of ``length`` bytes."""
    return _hashlib.shake_256(_to_bytes(data)).digest(length)


def blake2b(data) -> str:
    """BLAKE2b hex digest."""
    return _hashlib.blake2b(_to_bytes(data)).hexdigest()


def digest(algorithm: str, data) -> str:
    """Hex digest using any ``hashlib`` algorithm name (e.g. ``"sha3_256"``)."""
    try:
        h = _hashlib.new(algorithm, _to_bytes(data))
    except ValueError as exc:
        raise ValueError(f"unknown hash algorithm: {algorithm!r}") from exc
    return h.hexdigest()


def available_hashes():
    """Sorted list of supported hash algorithm names."""
    return sorted(_hashlib.algorithms_available)


# ============================================================================
# HMAC and key derivation
# ============================================================================

def hmac_sha256(key, message) -> str:
    """HMAC-SHA256 hex tag of ``message`` under ``key``."""
    return _hmac.new(_to_bytes(key), _to_bytes(message), _hashlib.sha256).hexdigest()


def hmac_sha3_256(key, message) -> str:
    """HMAC-SHA3-256 hex tag (post-quantum-resistant hash)."""
    return _hmac.new(_to_bytes(key), _to_bytes(message), _hashlib.sha3_256).hexdigest()


def hmac_sha512(key, message) -> str:
    """HMAC-SHA512 hex tag."""
    return _hmac.new(_to_bytes(key), _to_bytes(message), _hashlib.sha512).hexdigest()


def hkdf_sha256(ikm, length=32, salt=b"", info=b"") -> bytes:
    """HKDF-SHA256 extract-and-expand, returning ``length`` bytes.

    RFC 5869 caps the output at ``255 * HashLen`` bytes (8160 for SHA-256);
    the counter is a single byte, so a larger request is rejected clearly
    instead of failing later with an obscure ``ValueError``.
    """
    max_length = 255 * _hashlib.sha256().digest_size
    if length < 0:
        raise ValueError("hkdf_sha256 length must be non-negative")
    if length > max_length:
        raise ValueError(
            f"hkdf_sha256 cannot produce {length} bytes; the maximum for "
            f"SHA-256 is {max_length}")
    ikm = _to_bytes(ikm)
    salt = _to_bytes(salt) if salt else b"\x00" * _hashlib.sha256().digest_size
    info = _to_bytes(info)
    prk = _hmac.new(salt, ikm, _hashlib.sha256).digest()

    output = b""
    block = b""
    counter = 1
    while len(output) < length:
        block = _hmac.new(
            prk, block + info + bytes([counter]), _hashlib.sha256
        ).digest()
        output += block
        counter += 1
    return output[:length]


# ============================================================================
# Randomness and comparison
# ============================================================================

def random_bytes(n: int) -> bytes:
    """Return ``n`` cryptographically secure random bytes."""
    if n < 0:
        raise ValueError("random_bytes length must be non-negative")
    return _secrets.token_bytes(n)


def random_hex(n: int) -> str:
    """Return ``n`` random bytes as a hex string (2*n characters)."""
    return _secrets.token_hex(n)


def random_int(bits: int = 256) -> int:
    """Return a secure random integer with ``bits`` bits."""
    return _secrets.randbits(bits)


def constant_time_compare(a, b) -> bool:
    """Compare two byte strings without leaking timing information."""
    return _secrets.compare_digest(_to_bytes(a), _to_bytes(b))


# ============================================================================
# Post-quantum key encapsulation (ML-KEM / Kyber)
# ============================================================================

def kem_keypair(algorithm: str = "ML-KEM-768") -> dict:
    """Generate a KEM key pair.

    Returns ``{"algorithm", "public_key", "secret_key"}`` as bytes.
    """
    return AuraDict(_backend.kem_keypair(algorithm))


def kem_encapsulate(public_key, algorithm: str = "ML-KEM-768") -> dict:
    """Encapsulate a shared secret to ``public_key``.

    Returns ``{"algorithm", "ciphertext", "shared_secret"}``.
    """
    return AuraDict(_backend.kem_encapsulate(public_key, algorithm))


def kem_decapsulate(secret_key, ciphertext, algorithm: str = "ML-KEM-768") -> bytes:
    """Recover the shared secret from ``ciphertext`` using ``secret_key``."""
    return _backend.kem_decapsulate(secret_key, ciphertext, algorithm)


# ============================================================================
# Post-quantum signatures (ML-DSA / Dilithium, SLH-DSA / SPHINCS+)
# ============================================================================

def dsa_keypair(algorithm: str = "ML-DSA-65") -> dict:
    """Generate a signature key pair.

    Returns ``{"algorithm", "public_key", "secret_key"}`` as bytes.
    """
    return AuraDict(_backend.dsa_keypair(algorithm))


def dsa_sign(secret_key, message, algorithm: str = "ML-DSA-65") -> bytes:
    """Sign ``message`` with ``secret_key``."""
    return _backend.dsa_sign(secret_key, message, algorithm)


def dsa_verify(public_key, message, signature, algorithm: str = "ML-DSA-65") -> bool:
    """Verify ``signature`` over ``message`` with ``public_key``."""
    return _backend.dsa_verify(public_key, message, signature, algorithm)


# ============================================================================
# Helpers
# ============================================================================

def _to_bytes(data) -> bytes:
    if isinstance(data, bytes):
        return data
    if isinstance(data, bytearray):
        return bytes(data)
    if isinstance(data, str):
        return data.encode("utf-8")
    raise TypeError("expected str or bytes")


def algorithms():
    """List the post-quantum algorithms the active backend supports."""
    return list(_backend.info()["algorithms"])
