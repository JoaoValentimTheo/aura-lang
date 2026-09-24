# Your first program

This walkthrough builds a small program piece by piece. You can follow along in
the [Playground](/playground/).

## 1. Print a line

```aura
fn main() {
    print("hello, Aura")
}
```

`fn main()` is the entry point. `print` writes its arguments, separated by
spaces, followed by a newline. A runnable program needs a `main`; a module of
declarations does not.

## 2. Bindings and mutation

`let` binds a name immutably. `let mut` allows reassignment.

```aura
fn main() {
    let name = "Aura"       # immutable
    let mut count = 0       # mutable
    count = count + 1
    print(f"{name}: {count}")
}
```

F-strings interpolate values between braces. `{name}` and `{count}` are
evaluated and formatted.

## 3. Functions

A function takes parameters and optionally declares an argument and return type.
The body's last statement is the value, so an explicit `return` is optional.

```aura
fn double(x) {
    x * 2
}

fn add(a: int, b: int) -> int {
    return a + b
}

fn main() {
    print(double(21))
    print(add(2, 3))
}
```

The annotations on `add` are checked where the checker can prove a mismatch;
`double` opts out and relies on the runtime.

## 4. Collections and iteration

```aura
fn main() {
    let xs = [3, 1, 2]
    let sorted = xs.sort()
    for x in sorted {
        print(x)
    }

    let capitals = {"fr": "Paris", "jp": "Tokyo"}
    capitals["br"] = "Brasilia"
    print(len(capitals))
}
```

Lists are ordered and mutable; maps are string-keyed and ordered by key. `for`
iterates a list's elements, a map's keys, a string's characters, or the integers
of a range.

## 5. Data and matching

```aura
struct Point { x: int, y: int }

enum Shape {
    Circle(int),
    Rectangle(int, int),
}

fn area(s) -> int {
    return match s {
        Circle(r) -> 3 * r * r
        Rectangle(w, h) -> w * h
    }
}

fn main() {
    let p = Point { x: 3, y: 4 }
    print(f"({p.x}, {p.y})")
    print(area(Shape.Circle(2)))
}
```

`match` destructures enum variants by name and binds their payload.

## 6. Errors

Only an explicit `throw` is catchable. Runtime diagnostics such as division by
zero are fatal and not catchable.

```aura
fn main() {
    try {
        throw "something went wrong"
    } catch e -> {
        print(f"caught: {e}")
    } finally {
        print("always runs")
    }
}
```

## Where to go next

* [Values and bindings](/docs/guide-basics/)
* [Functions and closures](/docs/guide-functions/)
* [Structs and enums](/docs/guide-data/)
* The full [examples catalog](/examples/)
