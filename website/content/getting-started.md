# Getting started

Aura is a small, dynamically-typed, expression-oriented scripting language. This
page gets you from zero to a running program in a few minutes, either in the
browser or on your machine.

## The fastest way: the Playground

The [Playground](/playground/) runs the real Aura WebAssembly runtime inside a
Web Worker — no installation, no account, and no code sent to a server. Pick a
runtime version, edit the source, and press **Run**.

```aura
fn main() {
    print("hello, Aura")
}
```

## The local way

Aura is built with Rust. If you have a recent stable Rust toolchain:

```bash
# clone and build the pure-Rust binary (no Python required)
git clone https://github.com/JoaoValentimTheo/aura-lang
cd aura-lang
cargo build --release --no-default-features --features cli,repl,json,regex,time

# run a program
./target/release/aura run examples/tour.aura

# or an interactive session
./target/release/aura repl
```

See [Installing Aura](/docs/install/) for the full details, including the
optional Python bridge.

## The shape of an Aura program

A runnable program declares `fn main()`. Top-level items are functions, structs,
enums, type aliases, constants, and expressions. Declarations are hoisted, so
functions may refer to each other freely, including mutually.

```aura
fn is_even(n) -> bool {
    return n % 2 == 0
}

fn main() {
    let xs = [1, 2, 3, 4, 5, 6]
    let evens = xs.filter(is_even)
    print(evens)
}
```

## What to read next

* **New to programming or to Aura?** Follow [Your first program](/docs/first-program/) and then the language guide.
* **Want the precise rules?** The [language specification](https://github.com/JoaoValentimTheo/aura-lang/blob/main/docs/LANGUAGE_SPEC.md) is normative.
* **Looking for a specific function?** The [standard library reference](/docs/reference-stdlib/) lists every builtin.

> Aura's diagnostic codes are stable and part of the public contract. When you
> see `E3001` (type mismatch) or `E2003` (undefined name), the
> [diagnostics reference](/docs/reference-errors/) explains exactly what it means.
