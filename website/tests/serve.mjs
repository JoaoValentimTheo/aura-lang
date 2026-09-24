// Static server for website tests and manual preview.
//
// Serves the built site from `website/dist/`. Supports a base prefix so a
// non-root build can be previewed at the same path it will be deployed to.
//
// Usage: node website/tests/serve.mjs [port] [--base=/prefix/] [root]

import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const portArg = args.find((a) => /^\d+$/.test(a));
const baseArg = args.find((a) => a.startsWith("--base="));
const base = baseArg ? baseArg.slice("--base=".length) : "/";
const rootArg = args.find((a) => !/^\d+$/.test(a) && !a.startsWith("--"));
const root = resolve(rootArg || join(here, "..", "dist"));

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".xml": "application/xml",
  ".txt": "text/plain; charset=utf-8",
};

export function startServer(port = 0) {
  const b = base.endsWith("/") ? base : `${base}/`;
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url, "http://localhost");
      let pathname = decodeURIComponent(url.pathname);
      // Strip the configured base prefix.
      if (b !== "/") {
        if (!pathname.startsWith(b)) {
          res.writeHead(404).end("not found");
          return;
        }
        pathname = "/" + pathname.slice(b.length);
      }
      if (pathname === "/") pathname = "/index.html";
      const target = join(root, pathname.replace(/^(\.\.[/\\])+/, ""));
      if (!target.startsWith(root)) {
        res.writeHead(403).end("forbidden");
        return;
      }
      let info = await stat(target).catch(() => null);
      let finalTarget = target;
      if (info && info.isDirectory()) {
        finalTarget = join(target, "index.html");
        info = await stat(finalTarget).catch(() => null);
      }
      if (!info) {
        // Serve the 404 page like GitHub Pages.
        const notFound = join(root, "404.html");
        const body = await readFile(notFound).catch(() => Buffer.from("not found"));
        res.writeHead(404, { "content-type": TYPES[".html"] });
        res.end(body);
        return;
      }
      const body = await readFile(finalTarget);
      res.writeHead(200, {
        "content-type": TYPES[extname(finalTarget)] || "application/octet-stream",
      });
      res.end(body);
    } catch (err) {
      res.writeHead(500).end(String(err));
    }
  });
  return new Promise((resolveServer) => {
    server.listen(port, "127.0.0.1", () => {
      resolveServer({ server, port: server.address().port, root, base: b });
    });
  });
}

const isMain =
  process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url));
if (isMain) {
  const { port: actual, base: b } = await startServer(Number(portArg || 8080));
  console.log(`Aura website: http://127.0.0.1:${actual}${b} (root ${root})`);
}
