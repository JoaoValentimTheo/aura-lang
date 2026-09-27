// Substrate stack calibration for the WebAssembly runtime.
//
// On `wasm32-unknown-unknown` the Rust stack lives in linear memory and is
// sized by the linker (default 1 MiB). Aura's parser has a substrate-calibrated
// recursion backstop (`aura::parse::parse_recursion_budget`): **768** frames on
// wasm, chosen so every AST-valid program is accepted while over-deep input is
// reported as `E1015` rather than a host failure (`docs/LANGUAGE_SPEC.md` §31.2,
// §31.5).
//
// The most frame-expensive recursive path is a nested call argument
// (`f(f(f(…)))`), which recurses through
// `expr` → `unary` → `postfix` → `atom` → `call_args` → `cons_arg` → `expr`.
// On the default 1 MiB linear stack that path exhausted the stack at roughly
// 907 frames; the backstop is therefore set to 768 — *below* that physical
// ceiling — so over-deep input yields `E1015` before the stack can trap.
// (Native runs the parser on a dedicated 64 MiB stack and always reported
// `E1015`.)
//
// The larger linear stack reserved below is belt-and-braces: it keeps the
// backstop comfortably clear of the physical ceiling and leaves margin for
// future changes in parser frame size, without itself defining any language
// limit. The authoritative bound remains the 768-frame backstop, which is
// language-level and identical on every substrate's *observed* behavior
// (over-limit input is `E1015`, never a trap). Applied only to the wasm target.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size=4194304");
    }
}
