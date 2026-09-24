// A tiny static file server for Playground tests and manual viewing.
//
// Usage: node playground/tests/node/serve.mjs [port] [root]
//
// The root defaults to the `playground/` directory, matching the layout the
// Playground expects. `.wasm` is served as application/wasm and `.mjs`/`.js`
// as text/javascript so module workers load correctly.

import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, normalize, join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const defaultRoot = resolve(here, "../..");
const root = resolve(process.argv[3] || defaultRoot);

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
};

export function startServer(port = 0) {
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url, "http://localhost");
      let pathname = decodeURIComponent(url.pathname);
      if (pathname === "/") pathname = "/index.html";
      const target = join(root, normalize(pathname).replace(/^(\.\.[/\\])+/, ""));
      if (!target.startsWith(root)) {
        res.writeHead(403).end("forbidden");
        return;
      }
      const info = await stat(target).catch(() => null);
      if (!info || info.isDirectory()) {
        res.writeHead(404).end("not found");
        return;
      }
      const body = await readFile(target);
      res.writeHead(200, {
        "content-type": TYPES[extname(target)] || "application/octet-stream",
      });
      res.end(body);
    } catch (err) {
      res.writeHead(500).end(String(err));
    }
  });
  return new Promise((resolveServer) => {
    server.listen(port, "127.0.0.1", () => {
      const addr = server.address();
      resolveServer({ server, port: addr.port, root });
    });
  });
}

const isMain = process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url));
if (isMain) {
  const port = Number(process.argv[2] || 8080);
  const { port: actual } = await startServer(port);
  console.log(`Aura Playground: http://127.0.0.1:${actual}/ (root ${root})`);
}
