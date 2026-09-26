// Substrate stack calibration for the WebAssembly runtime.
//
// On `wasm32-unknown-unknown` the Rust stack lives in linear memory and is
// sized by the linker (default 1 MiB). Aura's parser has a substrate-calibrated
// recursion backstop (`aura::parse::parse_recursion_budget`): 1024 frames on
// wasm, chosen so every AST-valid program is accepted while over-deep input is
// reported as `E1015` rather than a host failure (`docs/LANGUAGE_SPEC.md` §31.2,
// §31.5).
//
// The most frame-expensive recursive path is a nested call argument
// (`f(f(f(…)))`), which recurses through
// `expr` → `unary` → `postfix` → `atom` → `call_args` → `cons_arg` → `expr`.
// On the default 1 MiB linear stack that path exhausts the stack at ~907
// frames — *below* the 1024-frame backstop — so an over-deep program trapped
// with `memory access out of bounds` instead of reporting `E1015`. Native runs
// the parser on a dedicated 64 MiB stack and always reported `E1015`, making
// this a native/wasm divergence.
//
// Reserve a larger linear stack so the existing parser backstop is reachable on
// every recursive path and the trap becomes `E1015`. This does not change the
// language's semantic nesting limit; it only lets the wasm substrate reach the
// limit the language already defines. Applied only to the wasm target.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size=4194304");
    }
}
