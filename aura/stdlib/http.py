"""Aura Standard Library - HTTP client module.

Uses Python's standard-library :mod:`urllib` so it works with no third-party
dependency. If ``requests`` is installed it is used automatically for a
nicer API; otherwise the urllib fallback is transparently selected.

Only client-side requests are exposed; there is no server.

Security note: only ``http`` and ``https`` URLs are accepted. Other schemes
(notably ``file://``, ``ftp://`` and ``data:``) are rejected with a
``ValueError`` so a request cannot silently read local files or reach an
unexpected protocol. Loopback, link-local and private (RFC 1918) hosts are
also rejected to limit server-side request forgery (SSRF) against local
services and cloud metadata endpoints. The check **fails closed**: a hostname
that does not resolve is treated as blocked rather than being allowed, and
IPv4-mapped IPv6 addresses are classified by their embedded IPv4 address. Set
``AURA_HTTP_ALLOW_PRIVATE=1`` to opt out when a program genuinely needs to call
a private address.
"""

import asyncio as _asyncio
import ipaddress as _ipaddress
import json as _json
import os as _os
import socket as _socket
import urllib.error as _urlerror
import urllib.parse as _urlparse
import urllib.request as _urlrequest

from .collections import AuraDict

_ALLOWED_SCHEMES = ('http', 'https')

# Upper bound on a response body (default 32 MiB) so a server cannot exhaust
# memory. Set AURA_HTTP_MAX_BYTES to override (0 disables the limit).
_DEFAULT_MAX_BYTES = 32 * 1024 * 1024


def _max_bytes():
    raw = _os.environ.get('AURA_HTTP_MAX_BYTES')
    if raw is None:
        return _DEFAULT_MAX_BYTES
    try:
        return int(raw)
    except ValueError:
        return _DEFAULT_MAX_BYTES


def _allow_private():
    return _os.environ.get('AURA_HTTP_ALLOW_PRIVATE') == '1'


def _is_blocked_host(hostname):
    """Return True when ``hostname`` resolves to a non-public address.

    Fails **closed**: a hostname that cannot be resolved is treated as blocked,
    because an unresolvable name cannot be proven public and would otherwise be
    resolved again (to anything) by the HTTP client at connect time. IPv4-mapped
    IPv6 addresses (``::ffff:127.0.0.1``) are unwrapped so they are classified
    by their embedded IPv4 address.
    """
    if not hostname:
        return True
    try:
        addresses = [hostname]
        _ipaddress.ip_address(hostname)
    except ValueError:
        try:
            addresses = [info[4][0] for info in _socket.getaddrinfo(hostname, None)]
        except _socket.gaierror:
            # Cannot resolve: refuse rather than let the client try later.
            return True
        except (UnicodeError, OSError):
            return True
    if not addresses:
        return True
    for address in addresses:
        try:
            addr = _ipaddress.ip_address(address)
        except ValueError:
            return True
        if isinstance(addr, _ipaddress.IPv6Address) and addr.ipv4_mapped is not None:
            addr = addr.ipv4_mapped
        if (addr.is_private or addr.is_loopback or addr.is_link_local
                or addr.is_reserved or addr.is_multicast or addr.is_unspecified):
            return True
    return False


def _validate_url(url):
    """Reject non-HTTP(S) URLs before any network or file access happens."""
    if not isinstance(url, str) or not url:
        raise ValueError("URL must be a non-empty string")
    parsed = _urlparse.urlparse(url)
    scheme = (parsed.scheme or '').lower()
    if scheme not in _ALLOWED_SCHEMES:
        raise ValueError(
            f"unsupported URL scheme {scheme!r}; only http and https are allowed"
        )
    if not parsed.netloc:
        raise ValueError(f"URL has no host: {url!r}")
    if not _allow_private():
        hostname = parsed.hostname
        if hostname and _is_blocked_host(hostname):
            raise ValueError(
                f"requests to private or local address {hostname!r} are blocked; "
                "set AURA_HTTP_ALLOW_PRIVATE=1 to allow"
            )
    return url


def _response(status, body, headers, ok, url):
    # AuraDict lets Aura code use `response.status` and `response["status"]`.
    return AuraDict({
        'status': status,
        'body': body,
        'headers': headers,
        'ok': ok,
        'url': url,
    })


def _use_requests():
    try:
        import requests  # noqa: F401
        return True
    except ImportError:
        return False


class _SafeRedirectHandler(_urlrequest.HTTPRedirectHandler):
    """Re-validate every redirect hop so an open redirect cannot reach a
    private host after the initial URL passed validation."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        _validate_url(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def _read_limited(stream, max_bytes):
    """Read at most ``max_bytes`` from ``stream`` (0/None means unlimited)."""
    if not max_bytes or max_bytes <= 0:
        return stream.read()
    return stream.read(max_bytes)


def _json_headers(headers, data):
    merged = dict(headers or {})
    if data is not None:
        merged.setdefault('Content-Type', 'application/json')
    return merged


def _request_with_requests(method, url, data, headers, timeout, max_bytes):
    """``requests`` backend that follows redirects with SSRF re-validation.

    Redirects are followed manually so every hop is re-validated (unlike
    ``requests``' own handler, which would happily land on a private host).
    The body is streamed and capped, so an oversized response cannot exhaust
    memory before it is rejected.
    """
    import requests

    req_data = data
    if data is not None and not isinstance(data, (str, bytes)):
        req_data = _json.dumps(data)

    current = url
    for _ in range(10):  # bounded redirect chain
        _validate_url(current)
        response = requests.request(
            method.upper(), current,
            data=req_data, headers=_json_headers(headers, data),
            timeout=timeout, allow_redirects=False, stream=True,
        )
        if response.status_code in (301, 302, 303, 307, 308):
            location = response.headers.get('Location')
            response.close()
            if not location:
                break
            current = _urlparse.urljoin(current, location)
            # A redirect turns the request into a GET for 301/302/303.
            if response.status_code in (301, 302, 303):
                method = 'GET'
                req_data = None
            continue
        content = _read_limited(response.raw, max_bytes)
        text = content.decode('utf-8', errors='replace')
        return _response(
            response.status_code, text, dict(response.headers),
            response.ok, response.url,
        )
    raise _urlerror.URLError("too many redirects")


def request(method, url, data=None, headers=None, timeout=30):
    """Perform an HTTP request and return a response-like dict.

    The returned mapping has ``status``, ``body`` (text), ``headers`` and
    ``ok`` keys, so it works the same whether ``requests`` or ``urllib`` is
    used underneath.

    Redirects are not followed blindly: each hop is re-validated (scheme and
    private-host checks), so an open redirect cannot bypass the SSRF guard.
    Response bodies are capped (see ``AURA_HTTP_MAX_BYTES``).
    """
    _validate_url(url)
    max_bytes = _max_bytes()
    if _use_requests():
        return _request_with_requests(
            method, url, data, headers, timeout, max_bytes)

    payload = None
    if data is not None:
        payload = data.encode('utf-8') if isinstance(data, str) else _json.dumps(data).encode('utf-8')
    req = _urlrequest.Request(url, data=payload, method=method.upper(),
                              headers=_json_headers(headers, data))
    opener = _urlrequest.build_opener(_SafeRedirectHandler())
    try:
        with opener.open(req, timeout=timeout) as resp:
            body = _read_limited(resp, max_bytes).decode('utf-8', errors='replace')
            return _response(resp.status, body, dict(resp.headers),
                             200 <= resp.status < 300, resp.geturl())
    except _urlerror.HTTPError as exc:
        body = _read_limited(exc, max_bytes).decode('utf-8', errors='replace')
        return _response(exc.code, body, dict(exc.headers), False, url)


def get(url, headers=None, timeout=30):
    """HTTP GET."""
    return request('GET', url, headers=headers, timeout=timeout)


def post(url, data=None, headers=None, timeout=30):
    """HTTP POST."""
    return request('POST', url, data=data, headers=headers, timeout=timeout)


def put(url, data=None, headers=None, timeout=30):
    """HTTP PUT."""
    return request('PUT', url, data=data, headers=headers, timeout=timeout)


def delete(url, headers=None, timeout=30):
    """HTTP DELETE."""
    return request('DELETE', url, headers=headers, timeout=timeout)


def _as_aura(obj):
    """Recursively wrap JSON mappings as AuraDict for attribute access."""
    if isinstance(obj, dict):
        return AuraDict({k: _as_aura(v) for k, v in obj.items()})
    if isinstance(obj, list):
        return [_as_aura(v) for v in obj]
    return obj


def get_json(url, headers=None, timeout=30):
    """HTTP GET and parse the response body as JSON."""
    response = request('GET', url, headers=headers, timeout=timeout)
    return _as_aura(_json.loads(response['body']))


def post_json(url, data, headers=None, timeout=30):
    """HTTP POST a JSON body and parse the JSON response."""
    body = _json.dumps(data)
    merged = dict(headers or {})
    merged.setdefault('Content-Type', 'application/json')
    response = request('POST', url, data=body, headers=merged, timeout=timeout)
    if not response['body']:
        return None
    return _as_aura(_json.loads(response['body']))


def quote(text):
    """Percent-encode ``text`` for use in a URL."""
    return _urlparse.quote(text)


def unquote(text):
    """Decode a percent-encoded URL component."""
    return _urlparse.unquote(text)


def build_url(base, params=None):
    """Append query parameters to ``base``."""
    if not params:
        return base
    sep = '&' if '?' in base else '?'
    return base + sep + _urlparse.urlencode(params)


# ============================================================================
# Async variants
# ============================================================================
#
# Aura's HTTP client is synchronous and vetted (scheme allow-list, SSRF guard,
# bounded redirects, capped body). Rather than reimplement all of that against
# a third-party async client, the async entry points await the synchronous
# request on a worker thread. Every guard therefore applies unchanged.

async def arequest(method, url, data=None, headers=None, timeout=30):
    """Await :func:`request` off the event loop."""
    return await _asyncio.to_thread(
        request, method, url, data, headers, timeout)


async def aget(url, headers=None, timeout=30):
    """Await :func:`get` off the event loop."""
    return await _asyncio.to_thread(get, url, headers, timeout)


async def apost(url, data=None, headers=None, timeout=30):
    """Await :func:`post` off the event loop."""
    return await _asyncio.to_thread(post, url, data, headers, timeout)


async def aput(url, data=None, headers=None, timeout=30):
    """Await :func:`put` off the event loop."""
    return await _asyncio.to_thread(put, url, data, headers, timeout)


async def adelete(url, headers=None, timeout=30):
    """Await :func:`delete` off the event loop."""
    return await _asyncio.to_thread(delete, url, headers, timeout)


async def aget_json(url, headers=None, timeout=30):
    """Await :func:`get_json` off the event loop."""
    return await _asyncio.to_thread(get_json, url, headers, timeout)


async def apost_json(url, data, headers=None, timeout=30):
    """Await :func:`post_json` off the event loop."""
    return await _asyncio.to_thread(post_json, url, data, headers, timeout)
