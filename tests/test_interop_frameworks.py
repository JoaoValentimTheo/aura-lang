"""Phase 1 — Python interop parity for framework-shaped apps.

Two layers of validation:

* **Stub tests** exercise the interop *surface* with lightweight modules that
  mirror the frameworks' shapes (dotted decorators, route registration,
  attribute assignment on foreign objects, base-class extension via the Python
  MRO, keyword/positional spread). They run everywhere, with no dependency.
* **Real-framework tests** run the same constructs against the actual Flask,
  Django and Flet packages when installed, as the end-to-end proof that the
  semantics hold for production code. A failure there is a semantic gap to fix.
"""

import contextlib
import io
import sys
import types
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Parser, Tokenizer  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def run_aura(source: str, stubs=None):
    """Transpile and execute ``source`` with optional stub modules installed."""
    saved = {}
    if stubs:
        for name, module in stubs.items():
            saved[name] = sys.modules.get(name)
            sys.modules[name] = module
    try:
        tokens = Tokenizer(source).tokenize()
        program = Parser(tokens).parse()
        code = Transformer().transform(program)
        namespace = {"__name__": "__aura_interop__"}
        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            exec(code, namespace)  # noqa: S102 - test harness
        return buffer.getvalue().strip(), code
    finally:
        if stubs:
            for name, original in saved.items():
                if original is None:
                    sys.modules.pop(name, None)
                else:
                    sys.modules[name] = original


def transpile(source: str) -> str:
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    return Transformer().transform(program)


def make_flask_stub():
    """A minimal Flask-shaped module: an app object with a route decorator."""
    module = types.ModuleType("flask")

    class Flask:
        def __init__(self, name="app"):
            self.name = name
            self.routes = {}

        def route(self, rule, methods=None):
            def decorator(fn):
                self.routes[rule] = (fn, methods or ["GET"])
                return fn
            return decorator

        def run(self, **kwargs):
            print(f"running {self.name} with {len(self.routes)} routes")

    module.Flask = Flask
    return module


def make_django_stub():
    """A minimal Django-shaped package: HttpResponse/JsonResponse and path()."""
    http = types.ModuleType("django.http")
    urls = types.ModuleType("django.urls")

    class HttpResponse:
        def __init__(self, body):
            self.body = body

    class JsonResponse:
        def __init__(self, data):
            self.data = data

    def path(route, view):
        return (route, view)

    http.HttpResponse = HttpResponse
    http.JsonResponse = JsonResponse
    urls.path = path

    django = types.ModuleType("django")
    django.http = http
    django.urls = urls
    return {"django": django, "django.http": http, "django.urls": urls}


def make_flet_stub():
    """A minimal Flet-shaped module: Text/ElevatedButton and page.add."""
    module = types.ModuleType("flet")

    class Control:
        def __init__(self, value=None, **kwargs):
            self.value = value
            for key, val in kwargs.items():
                setattr(self, key, val)

    class Text(Control):
        pass

    class ElevatedButton(Control):
        pass

    module.Text = Text
    module.ElevatedButton = ElevatedButton
    module.controls = []
    return module


# ============================================================================
# Dotted decorators (the Flask/FastAPI routing shape)
# ============================================================================

def test_dotted_decorator_parses_and_runs():
    flask = make_flask_stub()
    source = (
        "import flask\n"
        "let app = flask.Flask('demo')\n"
        "@app.route('/', methods=['GET'])\n"
        "def index() { return 'hi' }\n"
        "def main() { print(app.routes['/'][1][0]) }\n"
        "main()"
    )
    out, code = run_aura(source, stubs={"flask": flask})
    assert out == "GET"
    assert "@app.route('/', methods=['GET'])" in code


def test_dotted_decorator_with_keyword_arguments():
    flask = make_flask_stub()
    source = (
        "import flask\n"
        "let app = flask.Flask('demo')\n"
        "@app.route('/x', methods=['POST'])\n"
        "def handler() { return 'ok' }\n"
        "def main() { print(len(app.routes)) }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs={"flask": flask})
    assert out == "1"


def test_deeply_dotted_decorator():
    outer = types.ModuleType("outer")
    inner = types.ModuleType("outer.inner")

    def deco(fn):
        fn.tagged = True
        return fn

    inner.deco = deco
    outer.inner = inner
    source = (
        "import outer.inner\n"
        "@outer.inner.deco\n"
        "def target() { return 1 }\n"
        "def main() { print(target.tagged) }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs={"outer": outer, "outer.inner": inner})
    assert out == "True"


# ============================================================================
# Flask-shaped app end to end
# ============================================================================

def test_flask_shaped_app_with_stub():
    flask = make_flask_stub()
    source = (
        "import flask\n"
        "let app = flask.Flask('demo')\n"
        "@app.route('/', methods=['GET'])\n"
        "def index() { return 'Aura + Flask' }\n"
        "@app.route('/hello/<name>', methods=['GET'])\n"
        "def hello(name) { return f'Hello, {name}!' }\n"
        "def main() { app.run(host='127.0.0.1', port=5000) }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs={"flask": flask})
    assert out == "running demo with 2 routes"
    assert flask.Flask("x").route("/", methods=["GET"]) is not None


# ============================================================================
# Django-shaped module end to end
# ============================================================================

def test_django_shaped_views_and_urls():
    stubs = make_django_stub()
    source = (
        "import django.http\n"
        "from django.urls import path\n"
        "def home(request) { return django.http.HttpResponse('Aura + Django') }\n"
        "def health(request) { return django.http.JsonResponse({'status': 'ok'}) }\n"
        "let urlpatterns = [path('', home), path('health', health)]\n"
        "def main() { print(f'{len(urlpatterns)} routes registered') }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs=stubs)
    assert out == "2 routes registered"


def test_django_view_returns_response():
    stubs = make_django_stub()
    source = (
        "import django.http\n"
        "def home(request) { return django.http.HttpResponse('body') }\n"
        "def main() { print(home(None).body) }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs=stubs)
    assert out == "body"


# ============================================================================
# Flet-shaped UI end to end
# ============================================================================

def test_flet_shaped_ui_with_stub():
    flet = make_flet_stub()
    added = []

    class Page:
        def __init__(self):
            self.title = None

        def add(self, *controls):
            added.extend(controls)

        def update(self):
            pass

    source = (
        "import flet\n"
        "def main(page) {\n"
        "  page.title = 'Aura + Flet'\n"
        "  page.add(flet.Text('hi'), flet.ElevatedButton('go'))\n"
        "}\n"
        "main(page_stub)"
    )
    namespace_stub = Page()
    saved_page = sys.modules.get("page_stub")
    saved_flet = sys.modules.get("flet")
    sys.modules["page_stub"] = namespace_stub
    sys.modules["flet"] = flet
    try:
        tokens = Tokenizer(source).tokenize()
        code = Transformer().transform(Parser(tokens).parse())
        exec(code, {"__name__": "__aura_interop__",
                    "page_stub": namespace_stub})  # noqa: S102
    finally:
        if saved_page is None:
            sys.modules.pop("page_stub", None)
        else:
            sys.modules["page_stub"] = saved_page
        if saved_flet is None:
            sys.modules.pop("flet", None)
        else:
            sys.modules["flet"] = saved_flet

    assert namespace_stub.title == "Aura + Flet"
    assert len(added) == 2


# ============================================================================
# Framework base-class extension (Python MRO)
# ============================================================================

def test_class_extends_python_base():
    flask = make_flask_stub()
    source = (
        "import flask\n"
        "class App extends flask.Flask {\n"
        "  def __init__(self, name) { super(name) }\n"
        "}\n"
        "def main() { let a = App('child')\n print(a.name) }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs={"flask": flask})
    assert out == "child"


def test_class_extends_python_base_with_extra_fields():
    flask = make_flask_stub()
    source = (
        "import flask\n"
        "class App extends flask.Flask(debug_mode: bool = false) {\n"
        "  def describe(self) { return self.debug_mode }\n"
        "}\n"
        "def main() { print(App(true).describe()) }\n"
        "main()"
    )
    out, _ = run_aura(source, stubs={"flask": flask})
    assert out == "True"


# ============================================================================
# Spreads crossing the interop boundary
# ============================================================================

def test_kwargs_spread_into_framework_call():
    flask = make_flask_stub()
    source = (
        "import flask\n"
        "let app = flask.Flask('demo')\n"
        "let opts = {'methods': ['POST']}\n"
        "@app.route('/x', **opts)\n"
        "def handler() { return 'ok' }\n"
        "def main() { print(app.routes['/x'][1][0]) }\n"
        "main()"
    )
    out, code = run_aura(source, stubs={"flask": flask})
    assert out == "POST"
    assert "**" in code


def test_adaptive_spread_on_member_call():
    """`...value` on a member call picks kwargs for a dict, args otherwise."""
    from types import SimpleNamespace
    holder = SimpleNamespace(seen=None)

    def configure(**options):
        holder.seen = options

    holder.configure = configure
    source = (
        "def main() {\n"
        "  let opts = {'mode': 'fast'}\n"
        "  holder.configure(...opts)\n"
        "  print(holder.seen['mode'])\n"
        "}\n"
        "main()"
    )
    tokens = Tokenizer(source).tokenize()
    code = Transformer().transform(Parser(tokens).parse())
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, {"__name__": "__aura_interop__", "holder": holder})  # noqa: S102
    assert buffer.getvalue().strip() == "fast"
    assert "_aura_call" in code


def test_adaptive_spread_iterable_on_member_call():
    """A non-dict adaptive spread on a member call unpacks positionally."""
    from types import SimpleNamespace
    holder = SimpleNamespace(record=None)

    def collect(*values):
        holder.record = values

    holder.collect = collect
    source = (
        "def main() { holder.collect(...[1, 2, 3])\n print(len(holder.record)) }\n"
        "main()"
    )
    tokens = Tokenizer(source).tokenize()
    code = Transformer().transform(Parser(tokens).parse())
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, {"__name__": "__aura_interop__", "holder": holder})  # noqa: S102
    assert buffer.getvalue().strip() == "3"


def test_star_spread_into_framework_call():
    source = (
        "def collect(*args) { return args }\n"
        "def main() { let xs = [1, 2, 3]\n print(len(collect(*xs))) }\n"
        "main()"
    )
    out, _ = run_aura(source)
    assert out == "3"


# ============================================================================
# Examples in the repo transpile cleanly
# ============================================================================

def test_framework_examples_transpile_to_valid_python():
    examples = Path(__file__).parent.parent / "examples"
    for name in ("flask_app.aura", "django_views.aura", "flet_app.aura"):
        path = examples / name
        assert path.is_file(), f"missing example {name}"
        source = path.read_text()
        tokens = Tokenizer(source).tokenize()
        code = Transformer().transform(Parser(tokens).parse())
        compile(code, str(path), "exec")


# ============================================================================
# Real frameworks (skipped when not installed)
# ============================================================================
#
# The stub tests above pin the interop *surface*. When the real libraries are
# available, running the repo examples against them is the end-to-end proof that
# the semantics hold for production code: a failure here is a semantic gap to
# fix, not "this library does not work".

EXAMPLES = Path(__file__).parent.parent / "examples"


def _run_example(name):
    """Run an example with the real CLI and return its stdout."""
    import subprocess

    result = subprocess.run(
        [sys.executable, "-m", "aura.cli", "run", name],
        capture_output=True, text=True, timeout=60, cwd=str(EXAMPLES),
    )
    assert result.returncode == 0, (
        f"{name} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout.strip()


def test_real_flask_app_end_to_end():
    pytest.importorskip("flask")
    out = _run_example("flask_app.aura")
    assert "Aura + Flask" in out
    assert "Hello, Aura!" in out


def test_real_django_views_end_to_end():
    pytest.importorskip("django")
    out = _run_example("django_views.aura")
    assert out == "2 routes registered"


def test_real_flet_app_end_to_end():
    pytest.importorskip("flet")
    out = _run_example("flet_app.aura")
    assert out == "3 controls on 'Aura + Flet'"


def test_real_flask_routing_from_aura():
    """A Flask app defined in Aura serves through Flask's own test client."""
    pytest.importorskip("flask")
    source = (
        "import flask\n"
        "let app = flask.Flask(__name__)\n"
        "@app.route('/ping')\n"
        "def ping() { return 'pong' }\n"
        "def main() {\n"
        "  let client = app.test_client()\n"
        "  print(client.get('/ping').get_data(as_text=True))\n"
        "}\n"
        "main()"
    )
    out, _ = run_aura(source)
    assert out == "pong"


def test_real_django_http_response():
    django = pytest.importorskip("django")
    from django.conf import settings
    if not settings.configured:
        settings.configure(DEFAULT_CHARSET="utf-8")
    django.setup()
    source = (
        "from django.http import HttpResponse\n"
        "def main() { print(HttpResponse('ok').content.decode()) }\n"
        "main()"
    )
    out, _ = run_aura(source)
    assert out == "ok"


def test_real_flet_controls_build():
    pytest.importorskip("flet")
    source = (
        "import flet\n"
        "def main() {\n"
        "  let t = flet.Text('hi', size=20)\n"
        "  let b = flet.Button('go')\n"
        "  print(t.value, t.size, b.content)\n"
        "}\n"
        "main()"
    )
    out, _ = run_aura(source)
    assert out == "hi 20 go"
