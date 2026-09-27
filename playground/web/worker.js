// The Aura Playground execution Worker.
//
// Each run gets a *fresh* Worker created by the page and terminated on Stop or
// on completion. This is the primary hard-cancellation mechanism: terminating
// a Worker stops a synchronous wasm execution that would otherwise never
// return (`while true {}`). Cancellation is never turned into an Aura `throw`
// and is never catchable by Aura `try/catch`.
//
// The Worker is an orchestration layer only. It loads the selected, immutable
// runtime artifact, validates its ABI, calls it, and posts the structured
// result back. It implements no Aura semantics.
//
// Message in:  { runId, artifactUrl, expectedAbi, expectedSha256, source, args, stdin }
// Message out: { runId, kind: "loaded", runtimeVersion, abiVersion }
//              { runId, kind: "result", result }
//              { runId, kind: "error", phase, code, message }

importScripts(); // no-op; kept for clarity that there are no imports.

let runtimePromise = null;

async function loadRuntime(artifactUrl, expectedAbi, expectedSha256) {
  const url = new URL(artifactUrl, self.location.href);
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`cannot fetch runtime artifact (${response.status})`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  // `runtime.mjs` is a module; import it dynamically so this classic worker
  // keeps working from file servers that do not set module worker MIME types.
  const { AuraRuntime } = await import("./runtime.mjs");
  // Verify the bytes against the manifest's recorded SHA-256 *before*
  // compilation/instantiation. If the hash does not match, this throws a
  // structured `RuntimeIntegrityError` and no artifact is instantiated.
  const runtime = await AuraRuntime.fromBytes(bytes, artifactUrl, {
    expectedSha256: expectedSha256 || null,
  });
  if (expectedAbi !== null && expectedAbi !== undefined && runtime.abiVersion !== expectedAbi) {
    throw new Error(
      `runtime ABI ${runtime.abiVersion} does not match the expected ABI ${expectedAbi}`,
    );
  }
  return runtime;
}

self.onmessage = async (event) => {
  const msg = event.data || {};
  const { runId, artifactUrl, expectedAbi, expectedSha256, source, args, stdin } = msg;
  try {
    runtimePromise =
      runtimePromise || loadRuntime(artifactUrl, expectedAbi, expectedSha256);
    const runtime = await runtimePromise;
    self.postMessage({
      runId,
      kind: "loaded",
      runtimeVersion: runtime.runtimeVersion,
      languageVersion: runtime.languageVersion,
      abiVersion: runtime.abiVersion,
    });
    const result = runtime.run(source, { args: args || [], stdin: stdin ?? null });
    self.postMessage({ runId, kind: "result", result });
  } catch (err) {
    self.postMessage({
      runId,
      kind: "error",
      phase: "load",
      // A structured integrity failure carries its own code so the UI can
      // present it distinctly from a generic load error. Never leak raw
      // internals: only the message and code cross the boundary.
      code: (err && err.code) || "LOAD",
      message: err && err.message ? err.message : String(err),
    });
  }
};
