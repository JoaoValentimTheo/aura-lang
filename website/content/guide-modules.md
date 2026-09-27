# Modules and visibility

A `module` is a **real access boundary**. Its items are private by default and
exported with `pub`. A `::`-separated path or a `use` import reaches an
exported name. Modules are **in-source** — declared in the same file, not on
disk — which is what keeps Native and WebAssembly semantics identical, since
the WebAssembly host has no filesystem.

## Declaring and using a module

```aura
module shapes {
    pub struct Point { pub x: int, pub y: int }
    pub fn origin() -> Point { return Point { x: 0, y: 0 } }
    fn diagonal() -> int { return 1 }        # private to `shapes`
    pub fn area() -> int { return diagonal() }
}

fn main() {
    let p = shapes::origin()
    print(p.x)                                # 0
    print(shapes::area())                     # 1
}
```

`Point` and its fields are `pub`, so another module may name them. `diagonal`
is private, so it is reachable only from inside `shapes` — `area` can call it,
but `main` cannot.

## Private by default

```aura
module m {
    pub fn visible() -> int { return 1 }
    fn hidden() -> int { return 2 }
}

fn main() {
    print(m::visible())     # 1
    print(m::hidden())      # E2018: `hidden` is private to module `m`
}
```

The same rule covers **fields** and **methods**:

```aura
module counter {
    pub struct Counter { n: int }             # `n` is private
    pub fn make() -> Counter { return Counter { n: 0 } }
    impl Counter {
        pub fn get(self) -> int { return self.n }
        fn raw(self) -> int { return self.n }  # private
    }
}

fn main() {
    let c = counter::make()
    print(c.get())     # 0
    print(c.n)         # E2018: field `n` is private
}
```

A method provided by a trait follows the **trait's** visibility, not the
implementing struct's.

## Nested modules

Modules nest, and a descendant may reach an ancestor's private items — but not
the other way around.

```aura
module geometry {
    fn unit() -> int { return 1 }             # private to `geometry`
    module shapes {
        pub fn side() -> int { return unit() }  # a descendant may reach it
    }
}

fn main() {
    print(geometry::shapes::side())           # 1
}
```

## Imports

`use path` binds the imported item's last segment locally; `use path as Alias`
binds it under a new name.

```aura
module shapes {
    pub struct Point { pub x: int }
    pub fn origin() -> Point { return Point { x: 0 } }
}

use shapes::origin
use shapes::Point as P

fn main() {
    let p: P = origin()
    print(p.x)          # 0
}
```

An import may not silently shadow a name already declared in the module
(`E2007`); rename it with `as`.

## Visibility and mutation are separate

`pub` never grants mutation capability, and a mutable binding never bypasses
visibility.

```aura
module m {
    pub struct Account { pub balance: int }
    impl Account {
        pub fn deposit(mut self, amount: int) {
            self.balance = self.balance + amount
        }
    }
}

fn main() {
    let a = m::Account { balance: 0 }
    a.deposit(10)        # E2001: `a` is immutable (declare it `let mut`)
    let mut b = m::Account { balance: 0 }
    b.deposit(10)        # ok
    print(b.balance)     # 10
}
```

## Diagnostic codes

| Code | Meaning |
|---|---|
| `E2018` | A private item was accessed across a module boundary. |
| `E2019` | An unknown module, or an unknown item in a `use` path. |
| `E2007` | An import would shadow a name already declared in the module. |
| `E1006` | `pub` was used on an `impl` block, which has no name. |
