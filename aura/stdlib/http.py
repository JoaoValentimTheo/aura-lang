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
services and cloud metadata endpoints. Set ``AURA_HTTP_ALLOW_PRIVATE=1`` to
opt out when a program genuinely needs to call a private address.
"""

import ipaddress as _ipaddress
import json as _json
import os as _os
import socket as _socket
import urllib.error as _urlerror
import urllib.parse as _urlparse
import urllib.request as _urlrequest

from .collections import AuraDict


_ALLOWED_SCHEMES = ('http', 'https')


def _allow_private():
    return _os.environ.get('AURA_HTTP_ALLOW_PRIVATE') == '1'


def _is_blocked_host(hostname):
    """Return True when ``hostname`` resolves to a non-public address."""
    try:
        addresses = [hostname]
        _ipaddress.ip_address(hostname)
    except ValueError:
        try:
            addresses = [info[4][0] for info in _socket.getaddrinfo(hostname, None)]
        except _socket.gaierror:
            return False
    for address in addresses:
        try:
            addr = _ipaddress.ip_address(address)
        except ValueError:
            continue
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


def _json_headers(headers, data):
    merged = dict(headers or {})
    if data is not None:
        merged.setdefault('Content-Type', 'application/json')
    return merged


def request(method, url, data=None, headers=None, timeout=30):
    """Perform an HTTP request and return a response-like dict.

    The returned mapping has ``status``, ``body`` (text), ``headers`` and
    ``ok`` keys, so it works the same whether ``requests`` or ``urllib`` is
    used underneath.
    """
    _validate_url(url)
    if _use_requests():
        import requests
        # Non-string bodies are sent as JSON, matching the urllib path.
        req_data = data
        if data is not None and not isinstance(data, (str, bytes)):
            req_data = _json.dumps(data)
        response = requests.request(
            method.upper(), url,
            data=req_data, headers=_json_headers(headers, data), timeout=timeout,
        )
        return _response(
            response.status_code, response.text, dict(response.headers),
            response.ok, response.url,
        )

    payload = None
    if data is not None:
        payload = data.encode('utf-8') if isinstance(data, str) else _json.dumps(data).encode('utf-8')
    req = _urlrequest.Request(url, data=payload, method=method.upper(),
                              headers=_json_headers(headers, data))
    try:
        with _urlrequest.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode('utf-8', errors='replace')
            return _response(resp.status, body, dict(resp.headers),
                             200 <= resp.status < 300, resp.geturl())
    except _urlerror.HTTPError as exc:
        body = exc.read().decode('utf-8', errors='replace')
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