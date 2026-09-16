"""Backend selection for :mod:`aura.stdlib.crypto`.

Prefers a vetted implementation of the NIST post-quantum standards when the
optional ``cryptography`` package provides it:

* ML-KEM (FIPS 203, "Kyber")
* ML-DSA (FIPS 204, "Dilithium")
* SLH-DSA (FIPS 205, "SPHINCS+")

Falls back to a bundled pure-Python **reference** backend that is clearly
labelled non-production. See :func:`info`.
"""

import hashlib as _hashlib
import hmac as _hmac
import secrets as _secrets

# ============================================================================
# Production backend (cryptography>=44)
# ============================================================================

def _try_cryptography():
    try:
        from cryptography.hazmat.primitives.asymmetric import ml_dsa, ml_kem  # noqa: F401
    except Exception:
        return None
    return _CryptographyBackend()


class _CryptographyBackend:
    """ML-KEM/ML-DSA wrappers backed by the `cryptography` package."""

    name = "cryptography"
    production = True
    algorithms = ["ML-KEM-512", "ML-KEM-768", "ML-KEM-1024",
                  "ML-DSA-44", "ML-DSA-65", "ML-DSA-87"]

    def __init__(self):
        from cryptography.hazmat.primitives.asymmetric import ml_dsa, ml_kem

        self._ml_kem = ml_kem
        self._ml_dsa = ml_dsa

    def _kem_size(self, algorithm):
        mapping = {
            "ML-KEM-512": self._ml_kem.MLKEM512,
            "ML-KEM-768": self._ml_kem.MLKEM768,
            "ML-KEM-1024": self._ml_kem.MLKEM1024,
        }
        if algorithm not in mapping:
            raise ValueError(f"unsupported KEM algorithm: {algorithm!r}")
        return mapping[algorithm]

    def _dsa_size(self, algorithm):
        mapping = {
            "ML-DSA-44": self._ml_dsa.MLDSA44,
            "ML-DSA-65": self._ml_dsa.MLDSA65,
            "ML-DSA-87": self._ml_dsa.MLDSA87,
        }
        if algorithm not in mapping:
            raise ValueError(f"unsupported signature algorithm: {algorithm!r}")
        return mapping[algorithm]

    def kem_keypair(self, algorithm):
        private = self._kem_size(algorithm).generate_key_pair().private_key()
        return {
            "algorithm": algorithm,
            "public_key": private.public_key().public_bytes_raw(),
            "secret_key": private.private_bytes_raw(),
        }

    def kem_encapsulate(self, public_key, algorithm):
        pub = self._kem_size(algorithm).public_key_from_raw_bytes(public_key)
        shared, ciphertext = pub.encapsulate()
        return {"algorithm": algorithm, "ciphertext": ciphertext,
                "shared_secret": shared}

    def kem_decapsulate(self, secret_key, ciphertext, algorithm):
        priv = self._kem_size(algorithm).private_key_from_raw_bytes(secret_key)
        return priv.decapsulate(ciphertext)

    def dsa_keypair(self, algorithm):
        private = self._dsa_size(algorithm).generate_key_pair().private_key()
        return {
            "algorithm": algorithm,
            "public_key": private.public_key().public_bytes_raw(),
            "secret_key": private.private_bytes_raw(),
        }

    def dsa_sign(self, secret_key, message, algorithm):
        priv = self._dsa_size(algorithm).private_key_from_raw_bytes(secret_key)
        return priv.sign(_to_bytes(message))

    def dsa_verify(self, public_key, message, signature, algorithm):
        pub = self._dsa_size(algorithm).public_key_from_raw_bytes(public_key)
        try:
            pub.verify(signature, _to_bytes(message))
            return True
        except Exception:
            return False


# ============================================================================
# Reference backend (pure Python, NOT for production)
# ============================================================================

class _ReferenceBackend:
    """A pure-Python reference backend with correct API shapes.

    .. warning::

        This backend is a **demonstration**. It round-trips correctly but is
        **not cryptographically secure** and must never protect real secrets.
        Install ``cryptography>=44`` (the ``pqc`` extra) for a validated
        implementation.
    """

    name = "reference"
    production = False
    algorithms = ["ML-KEM-512", "ML-KEM-768", "ML-KEM-1024",
                  "ML-DSA-44", "ML-DSA-65", "ML-DSA-87"]

    def __init__(self):
        self._kem_sizes = {"ML-KEM-512": 32, "ML-KEM-768": 32, "ML-KEM-1024": 32}
        self._dsa_sizes = {"ML-DSA-44": 32, "ML-DSA-65": 48, "ML-DSA-87": 64}

    # -- KEM ---------------------------------------------------------------

    def _kem_secret_size(self, algorithm):
        if algorithm not in self._kem_sizes:
            raise ValueError(f"unsupported KEM algorithm: {algorithm!r}")
        return self._kem_sizes[algorithm]

    def kem_keypair(self, algorithm):
        size = self._kem_secret_size(algorithm)
        secret = _secrets.token_bytes(size)
        public = _hashlib.shake_256(b"pub" + secret).digest(size)
        return {"algorithm": algorithm, "public_key": public, "secret_key": secret}

    def kem_encapsulate(self, public_key, algorithm):
        size = self._kem_secret_size(algorithm)
        ephemeral = _secrets.token_bytes(size)
        # Derive a shared secret bound to both the ephemeral value and the
        # public key, then encapsulate the ephemeral value under the public key
        # with a SHAKE keystream.
        shared = _hashlib.shake_256(b"ss" + ephemeral + public_key).digest(32)
        keystream = _hashlib.shake_256(b"ks" + public_key).digest(size)
        ciphertext = bytes(a ^ b for a, b in zip(ephemeral, keystream, strict=False))
        return {"algorithm": algorithm, "ciphertext": ciphertext,
                "shared_secret": shared}

    def kem_decapsulate(self, secret_key, ciphertext, algorithm):
        size = self._kem_secret_size(algorithm)
        public = _hashlib.shake_256(b"pub" + secret_key).digest(size)
        keystream = _hashlib.shake_256(b"ks" + public).digest(size)
        ephemeral = bytes(a ^ b for a, b in zip(ciphertext, keystream, strict=False))
        return _hashlib.shake_256(b"ss" + ephemeral + public).digest(32)

    # -- signatures --------------------------------------------------------

    def _dsa_secret_size(self, algorithm):
        if algorithm not in self._dsa_sizes:
            raise ValueError(f"unsupported signature algorithm: {algorithm!r}")
        return self._dsa_sizes[algorithm]

    def dsa_keypair(self, algorithm):
        size = self._dsa_secret_size(algorithm)
        secret = _secrets.token_bytes(size)
        public = _hashlib.shake_256(b"vk" + secret).digest(size)
        return {"algorithm": algorithm, "public_key": public, "secret_key": secret}

    def dsa_sign(self, secret_key, message, algorithm):
        message = _to_bytes(message)
        size = self._dsa_secret_size(algorithm)
        public = _hashlib.shake_256(b"vk" + secret_key).digest(size)
        return _hmac.new(public, b"sig" + message, _hashlib.sha3_256).digest()

    def dsa_verify(self, public_key, message, signature, algorithm):
        message = _to_bytes(message)
        expected = _hmac.new(
            public_key, b"sig" + message, _hashlib.sha3_256
        ).digest()
        return _secrets.compare_digest(signature, expected)


# ============================================================================
# Selection
# ============================================================================

_BACKEND = None


def _select():
    global _BACKEND
    if _BACKEND is None:
        _BACKEND = _try_cryptography() or _ReferenceBackend()
    return _BACKEND


def info() -> dict:
    backend = _select()
    return {
        "name": backend.name,
        "production": backend.production,
        "algorithms": list(backend.algorithms),
    }


def kem_keypair(algorithm):
    return _select().kem_keypair(algorithm)


def kem_encapsulate(public_key, algorithm):
    return _select().kem_encapsulate(_to_bytes(public_key), algorithm)


def kem_decapsulate(secret_key, ciphertext, algorithm):
    return _select().kem_decapsulate(_to_bytes(secret_key), _to_bytes(ciphertext),
                                     algorithm)


def dsa_keypair(algorithm):
    return _select().dsa_keypair(algorithm)


def dsa_sign(secret_key, message, algorithm):
    return _select().dsa_sign(_to_bytes(secret_key), _to_bytes(message), algorithm)


def dsa_verify(public_key, message, signature, algorithm):
    return _select().dsa_verify(_to_bytes(public_key), _to_bytes(message),
                                _to_bytes(signature), algorithm)


def _to_bytes(data):
    if isinstance(data, bytes):
        return data
    if isinstance(data, bytearray):
        return bytes(data)
    if isinstance(data, str):
        return data.encode("utf-8")
    raise TypeError("expected str or bytes")
