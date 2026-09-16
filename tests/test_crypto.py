"""Tests for the cryptography stdlib, including the post-quantum backend.

These tests exercise the real (stdlib-backed) primitives and the pluggable
post-quantum facade. They do **not** assert that the reference backend is
secure — only that it round-trips and that production-backend gating works.
"""
import contextlib
import io
import os
import sys
import tempfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))


# ============================================================================
# Hashing
# ============================================================================

def test_sha2_digests():
    from aura.stdlib import crypto

    assert crypto.sha256("abc") == (
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    assert len(crypto.sha512("abc")) == 128


def test_sha3_and_shake():
    from aura.stdlib import crypto

    assert len(crypto.sha3_256("abc")) == 64
    assert len(crypto.sha3_512("abc")) == 128
    assert len(crypto.shake_256("abc", 16)) == 16


def test_blake2b_and_generic_digest():
    from aura.stdlib import crypto

    assert len(crypto.blake2b("abc")) == 128
    assert crypto.digest("sha3_256", "abc") == crypto.sha3_256("abc")


def test_digest_rejects_unknown_algorithm():
    from aura.stdlib import crypto

    with pytest.raises(ValueError):
        crypto.digest("not-a-hash", "x")


def test_available_hashes_nonempty():
    from aura.stdlib import crypto

    assert "sha256" in crypto.available_hashes()


def test_hash_accepts_bytes_and_str_equal():
    from aura.stdlib import crypto

    assert crypto.sha256(b"abc") == crypto.sha256("abc")


# ============================================================================
# HMAC / HKDF
# ============================================================================

def test_hmac_vectors():
    from aura.stdlib import crypto

    tag = crypto.hmac_sha256(b"key", "message")
    assert len(tag) == 64
    assert crypto.hmac_sha3_256(b"key", "message") != tag
    assert len(crypto.hmac_sha512(b"key", "message")) == 128


def test_hkdf_length_and_determinism():
    from aura.stdlib import crypto

    a = crypto.hkdf_sha256("ikm", 32, salt=b"salt", info=b"info")
    b = crypto.hkdf_sha256("ikm", 32, salt=b"salt", info=b"info")
    assert a == b
    assert len(a) == 32
    assert crypto.hkdf_sha256("ikm", 64) != a


# ============================================================================
# Randomness / comparison
# ============================================================================

def test_random_bytes_length_and_uniqueness():
    from aura.stdlib import crypto

    assert len(crypto.random_bytes(16)) == 16
    assert crypto.random_bytes(16) != crypto.random_bytes(16)
    with pytest.raises(ValueError):
        crypto.random_bytes(-1)


def test_random_hex_and_int():
    from aura.stdlib import crypto

    assert len(crypto.random_hex(8)) == 16
    assert crypto.random_int(8) < 256


def test_constant_time_compare():
    from aura.stdlib import crypto

    assert crypto.constant_time_compare(b"abc", b"abc") is True
    assert crypto.constant_time_compare(b"abc", b"abd") is False


# ============================================================================
# Backend selection
# ============================================================================

def test_backend_info_shape():
    from aura.stdlib import crypto

    info = crypto.backend_info()
    assert info["name"] in ("cryptography", "reference")
    assert isinstance(info["production"], bool)
    assert "ML-KEM-768" in info["algorithms"]


def test_require_production_backend_gates_reference():
    from aura.stdlib import crypto

    if crypto.is_production_backend():
        crypto.require_production_backend()  # no raise
    else:
        with pytest.raises(RuntimeError):
            crypto.require_production_backend()


def test_algorithms_available():
    from aura.stdlib import crypto

    algos = crypto.algorithms()
    assert "ML-KEM-768" in algos
    assert "ML-DSA-65" in algos


# ============================================================================
# Post-quantum KEM
# ============================================================================

@pytest.mark.parametrize("algorithm", ["ML-KEM-512", "ML-KEM-768", "ML-KEM-1024"])
def test_kem_roundtrip(algorithm):
    from aura.stdlib import crypto

    kp = crypto.kem_keypair(algorithm)
    assert kp["algorithm"] == algorithm
    enc = crypto.kem_encapsulate(kp["public_key"], algorithm)
    shared = crypto.kem_decapsulate(kp["secret_key"], enc["ciphertext"], algorithm)
    assert shared == enc["shared_secret"]
    assert len(shared) == 32


def test_kem_wrong_secret_differs():
    from aura.stdlib import crypto

    kp1 = crypto.kem_keypair("ML-KEM-768")
    kp2 = crypto.kem_keypair("ML-KEM-768")
    enc = crypto.kem_encapsulate(kp1["public_key"], "ML-KEM-768")
    wrong = crypto.kem_decapsulate(kp2["secret_key"], enc["ciphertext"], "ML-KEM-768")
    assert wrong != enc["shared_secret"]


def test_kem_rejects_unknown_algorithm():
    from aura.stdlib import crypto

    with pytest.raises(ValueError):
        crypto.kem_keypair("NOPE")


# ============================================================================
# Post-quantum signatures
# ============================================================================

@pytest.mark.parametrize("algorithm", ["ML-DSA-44", "ML-DSA-65", "ML-DSA-87"])
def test_signature_roundtrip(algorithm):
    from aura.stdlib import crypto

    kp = crypto.dsa_keypair(algorithm)
    sig = crypto.dsa_sign(kp["secret_key"], "payload", algorithm)
    assert crypto.dsa_verify(kp["public_key"], "payload", sig, algorithm) is True
    assert crypto.dsa_verify(kp["public_key"], "tampered", sig, algorithm) is False


def test_signature_wrong_key_fails():
    from aura.stdlib import crypto

    kp1 = crypto.dsa_keypair("ML-DSA-65")
    kp2 = crypto.dsa_keypair("ML-DSA-65")
    sig = crypto.dsa_sign(kp1["secret_key"], "m", "ML-DSA-65")
    assert crypto.dsa_verify(kp2["public_key"], "m", sig, "ML-DSA-65") is False


def test_signature_rejects_unknown_algorithm():
    from aura.stdlib import crypto

    with pytest.raises(ValueError):
        crypto.dsa_keypair("NOPE")


# ============================================================================
# End to end from Aura
# ============================================================================

def test_crypto_from_aura():
    from aura.parser.to_ast import parse_file
    from aura.transpiler.transformer import Transformer

    source = (
        "import stdlib.crypto as crypto\n"
        "let kp = crypto.kem_keypair()\n"
        "let enc = crypto.kem_encapsulate(kp.public_key)\n"
        "let shared = crypto.kem_decapsulate(kp.secret_key, enc.ciphertext)\n"
        "print(shared == enc.shared_secret)\n"
        "let sk = crypto.dsa_keypair()\n"
        "let sig = crypto.dsa_sign(sk.secret_key, 'm')\n"
        "print(crypto.dsa_verify(sk.public_key, 'm', sig))\n"
        "print(crypto.dsa_verify(sk.public_key, 'x', sig))\n"
    )
    with tempfile.NamedTemporaryFile("w", suffix=".aura", delete=False) as f:
        f.write(source)
        path = f.name
    try:
        program = parse_file(path)
        code = Transformer().transform(program)
        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            exec(compile(code, path, "exec"), {"__name__": "__crypto_test__"})
        assert buffer.getvalue() == "True\nTrue\nFalse\n"
    finally:
        os.unlink(path)