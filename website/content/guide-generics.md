# Generics

A generic declaration takes **type parameters**: placeholders substituted with
a concrete type before the program runs. Aura's generics are **static**,
**erased**, and **nominal**. The full architecture decision is
[`docs/GENERICS.md`](https://github.com/JoaoValentimTheo/aura-lang/blob/v0.3.1/docs/GENERICS.md);
the normative rules are in the specification (§36).

## Generic functions

A type parameter is written in angle brackets after the function's name, and a
bound with a colon:

```aura
fn identity<T>(x: T) -> T {
    return x
}

fn main() {
    print(identity(1))       # 1
    print(identity("a"))     # a
}
```

The argument type is **inferred**: `identity(1)` binds `T` to `int`,
`identity("a")` binds it to `string`. A parameter's name is not part of the
signature — `fn f<T>(x: T)` and `fn f<U>(x: U)` are the *same* overload, so
declaring both is `E2007`.

Explicit type arguments may be written when inference is not enough:

```aura
let x = identity<int>(1)
```

## Collections stay structural

Generic programming composes with Aura's existing collection syntax. There is
no `List<T>` or `Map<K, V>` built-in spelling:

```aura
fn first<T>(xs: [T]) -> T {
    return xs[0]
}

fn get<K, V>(m: {K: V}, key: K) -> V {
    return m[key]
}

type Map<K, V> = {K: V}
```

`[T]` is a list of `T`; `{K: V}` is a map of `V` keyed by `K`. `Map<K, V>` is
an ordinary alias over `{K: V}`, not new syntax.

The key position is **constrained**, generic or not. A map key must be
key-capable — `string`, `int`, or `bool`, or a union of these. A parameter used
as a key is admissible in the declaration but must resolve to a key-capable
type when instantiated:

```aura
let scores: Map<int, float> = {1: 9.5, 2: 8.75}   # ok
# let bad: Map<float, int> = {}                    # E3001: float is not key-capable
```

There is no explicit `Hashable`-style bound syntax; the constraint is implicit
in the map-key position (`LANGUAGE_SPEC.md` §5.2).

## Generic structs

A struct may take parameters, and construction infers them from the field
values:

```aura
struct Box<T> { value: T }

fn main() {
    let b = Box { value: 9 }
    print(b.value)           # 9

    let c = Box<int> { value: 1 }   # explicit
    print(c.value)           # 1
}
```

`Box<int>` and `Box<string>` are the same nominal `Box`. A wrong explicit
argument is rejected: `Box<int> { value: "x" }` is `E3001`.

## Generic methods

An `impl` block may declare parameters, which its methods use; a method may
add its own on top:

```aura
struct Box<T> { value: T }

impl<T> Box<T> {
    fn get(self) -> T { return self.value }
}

impl<T> Box<T> {
    fn replace<U>(self, v: U) -> U { return v }
}

fn main() {
    print(Box { value: 3 }.get())            # 3
    print(Box { value: 1 }.replace("z"))     # z
}
```

## Generic traits and bounds

A trait may be parameterised, and a type parameter may require a trait with a
**bound**:

```aura
trait Container<T> {
    fn get(self) -> T
}

struct Slot<T> { value: T }
impl<T> Container<T> for Slot<T> {
    fn get(self) -> T { return self.value }
}

fn unbox<T: Container<T>>(b: T) -> T {
    return b.get()
}

fn main() {
    print(unbox(Slot { value: 99 }))   # 99
}
```

A bound is a **static contract**: it lets the body call the trait's methods on
a `T`-typed value, and it is checked at the call site. A type that does not
implement the trait is `E3001`. There are no trait objects and no dynamic
dispatch.

## Generic aliases

An alias may take parameters and expands by substitution:

```aura
type Pair<T> = [T]

fn main() {
    let p: Pair<int> = [1, 2]
    print(p[1])              # 2
}
```

## How it runs

A type parameter is **erased before execution**. The runtime is already
dynamically typed, so a generic function is an ordinary function — there is no
monomorphization and no runtime dictionary. This keeps Native and WebAssembly
behaviour identical and resource use bounded.

## Rules at a glance

* Type parameters are static placeholders, substituted before execution.
* Identity is up to alpha-renaming: `f<T>` and `f<U>` are the same overload.
* Inference structurally matches argument types; an unbound parameter stays
  `Unknown`, never an arbitrary concrete type.
* A bound `T: Trait` is checked at the call site; there is no dynamic dispatch.
* `[T]` and `{K: V}` are the collection forms — no nominal `List`/`Map`. A map
  key must be key-capable (`string`, `int`, `bool`, or a union of these).
* A type parameter may not shadow a declared type or an outer parameter
  (`E2007`).
