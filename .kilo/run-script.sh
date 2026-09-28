#!/usr/bin/env bash
# Aura Agent Manager run script.
# Starts a local playground dev server for the selected worktree (or the main
# repo when LOCAL is selected), from WORKTREE_PATH with REPO_PATH set.
#
# The playground is a static site built from `website/` plus the runtime
# artifacts under `playground/runtimes/`. This script serves the built website
# so a reviewer can exercise the current runtime in a browser.
set -euo pipefail

ROOT="${WORKTREE_PATH:-$(pwd)}"
cd "$ROOT"

# Build the website (static output) if the builder exists.
if [ -f website/build.mjs ]; then
  echo "[aura-run] building website"
  node website/build.mjs
fi

# Derive a worktree-specific port so parallel worktrees do not collide.
BASE_PORT=4173
PORT="$BASE_PORT"
if [ -n "${WORKTREE_PATH:-}" ]; then
  HASH="$(printf '%s' "$WORKTREE_PATH" | cksum | awk '{print $1}')"
  PORT=$(( BASE_PORT + (HASH % 1000) ))
fi

SERVE_DIR="website/dist"
if [ ! -d "$SERVE_DIR" ]; then
  echo "[aura-run] no built site at $SERVE_DIR; serving repository root"
  SERVE_DIR="."
fi

echo "[aura-run] serving $SERVE_DIR on http://localhost:$PORT"
exec node -e '
  const http = require("http");
  const fs = require("fs");
  const path = require("path");
  const root = process.argv[1];
  const port = Number(process.argv[2]);
  const types = { ".html":"text/html", ".js":"text/javascript", ".css":"text/css",
                  ".json":"application/json", ".wasm":"application/wasm", ".mjs":"text/javascript" };
  http.createServer((req, res) => {
    let p = decodeURIComponent(req.url.split("?")[0]);
    if (p.endsWith("/")) p += "index.html";
    const file = path.join(root, p);
    fs.readFile(file, (err, data) => {
      if (err) { res.writeHead(404); res.end("not found"); return; }
      res.writeHead(200, { "content-type": types[path.extname(file)] || "application/octet-stream" });
      res.end(data);
    });
  }).listen(port);
' "$SERVE_DIR" "$PORT"
