// The Aura example catalog.
//
// Every program here is real, valid Aura. Some are marked `runnable`, meaning
// they can be sent to the Playground unchanged. The website build validates
// each `runnable` program against the actual WebAssembly runtime
// (`website/tests/validate-examples.mjs`), so a published example is a tested
// example.
//
// Fields:
//   id       stable identifier (used for anchors and links)
//   title    human name
//   level    "basics" | "intermediate" | "advanced"
//   tags     concept tags
//   summary  one-line description
//   source   the Aura program
//   output   the expected stdout (only for runnable examples)
//   note     optional explanation shown under the code

export const examples = [
  {
    id: "hello",
    title: "Hello, Aura",
    level: "basics",
    tags: ["io", "functions"],
    summary: "The smallest Aura program: a main function that prints a line.",
    output: "hello, Aura\n",
    source: `fn main() {
    print("hello, Aura")
}`,
    note: "Every runnable program declares `fn main()`. `print` writes a line of output.",
  },
  {
    id: "expressions",
    title: "Expressions and values",
    level: "basics",
    tags: ["values", "arithmetic"],
    summary: "Aura is expression-oriented; blocks and `if` yield values.",
    output: "7\nbigger\n3.5\n",
    source: `fn main() {
    let a = 1 + 2 * 3
    print(a)

    let label = if a > 5 { "bigger" } else { "smaller" }
    print(label)

    print(7.0 / 2.0)
}`,
    note: "An `if` expression yields the value of the branch it takes; a block yields its last statement.",
  },
  {
    id: "functions",
    title: "Functions",
    level: "basics",
    tags: ["functions", "recursion"],
    summary: "Named functions, optional type annotations, and recursion.",
    output: "fib(10) = 55\n",
    source: `fn fib(n) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(f"fib(10) = {fib(10)}")
}`,
    note: "Functions are hoisted and may be recursive. Annotations such as `-> int` are checked when the checker can prove a mismatch.",
  },
  {
    id: "collections",
    title: "Lists and maps",
    level: "basics",
    tags: ["collections", "iteration"],
    summary: "Ordered lists, string-keyed maps, and iteration.",
    output: "[1, 2, 3]\n3\nbr -> Brasilia\nfr -> Paris\njp -> Tokyo\n",
    source: `fn main() {
    let xs = [1, 2, 3]
    print(xs)
    print(len(xs))

    let capitals = {"fr": "Paris", "jp": "Tokyo"}
    capitals["br"] = "Brasilia"
    for k in capitals {
        print(f"{k} -> {capitals[k]}")
    }
}`,
    note: "Maps are ordered by key. `for` over a map yields its keys in ascending order.",
  },
  {
    id: "closures",
    title: "Closures",
    level: "intermediate",
    tags: ["functions", "closures"],
    summary: "Lambdas capture their defining environment by reference.",
    output: "15\n",
    source: `fn main() {
    let n = 10
    let add_n = (x) -> x + n
    print(add_n(5))
}`,
    note: "A lambda written `(x) -> expr` is a first-class value. Capture is by reference, not by value.",
  },
  {
    id: "pipeline",
    title: "Pipeline and higher-order functions",
    level: "intermediate",
    tags: ["pipeline", "collections"],
    summary: "Chain transformations with `|>` and list methods.",
    output: "[4, 16, 36]\n12\n",
    source: `fn main() {
    let xs = [1, 2, 3, 4, 5, 6]
    let squares_of_evens = xs |> filter((x) -> x % 2 == 0) |> map((x) -> x * x)
    print(squares_of_evens)

    print(xs.filter((x) -> x % 2 == 0).reduce((a, b) -> a + b, 0))
}`,
    note: "`x |> f(a)` passes `x` as the first argument of `f`. A pipeline must stay on one line.",
  },
  {
    id: "structs",
    title: "Structs",
    level: "intermediate",
    tags: ["structs", "data"],
    summary: "Named records with typed fields and named construction.",
    output: "origin = (0, 0)\ndistance^2 = 25\n",
    source: `struct Point { x: int, y: int }

fn dist_sq(p) -> int {
    return p.x * p.x + p.y * p.y
}

fn main() {
    let origin = Point { x: 0, y: 0 }
    print(f"origin = ({origin.x}, {origin.y})")

    let p = Point { x: 3, y: 4 }
    print(f"distance^2 = {dist_sq(p)}")
}`,
    note: "Fields are declared with types. A struct literal uses `Name { field: value }`.",
  },
  {
    id: "enums",
    title: "Enums and pattern matching",
    level: "intermediate",
    tags: ["enums", "match"],
    summary: "Variants with payloads, destructured by `match`.",
    output: "circle area ~ 12\nrect area = 12\n",
    source: `enum Shape {
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
    print(f"circle area ~ {area(Circle(2))}")
    print(f"rect area = {area(Rectangle(3, 4))}")
}`,
    note: "An enum variant pattern binds its payload. A `match` with no matching arm is `E4029`.",
  },
  {
    id: "destructuring",
    title: "Destructuring",
    level: "intermediate",
    tags: ["patterns", "destructuring"],
    summary: "Bind list and variant patterns with `let`.",
    output: "3\n30\n",
    source: `enum Pair { P(int, int) }

fn main() {
    let [a, b] = [1, 2]
    print(a + b)

    let P(x, y) = P(10, 20)
    print(x + y)
}`,
    note: "A destructuring `let` is atomic: a mismatch binds nothing and is `E3001`.",
  },
  {
    id: "errors",
    title: "Errors",
    level: "intermediate",
    tags: ["errors", "try"],
    summary: "Explicit `throw` is catchable; runtime errors are fatal.",
    output: "5\ncaught: division by zero\ndone\n",
    source: `fn safe_div(a, b) -> int {
    if b == 0 {
        throw "division by zero"
    }
    return a / b
}

fn main() {
    try {
        print(safe_div(10, 2))
        print(safe_div(1, 0))
    } catch e -> {
        print(f"caught: {e}")
    } finally {
        print("done")
    }
}`,
    note: "`try/catch` catches only explicit `throw`. A real division by zero (`1 / 0`) is `E4007` and is not catchable.",
  },
  {
    id: "args",
    title: "Command-line arguments",
    level: "intermediate",
    tags: ["io", "cli"],
    summary: "Read program arguments with `args()`.",
    output: "hello Ada\n",
    source: `fn main() {
    let who = args()[0]
    print(f"hello {who}")
}`,
    note: "In the Playground, supply one argument per line in the Arguments box.",
  },
  {
    id: "stdin",
    title: "Standard input",
    level: "intermediate",
    tags: ["io", "stdin"],
    summary: "Read standard input line by line until `none`.",
    output: "HELLO\nAURA\n",
    source: `fn main() {
    let mut line = read_line()
    while line != none {
        print(line.upper())
        line = read_line()
    }
}`,
    note: "Provide input in the Standard input box. `read_line()` returns `none` at end of input.",
  },
  {
    id: "json",
    title: "JSON",
    level: "advanced",
    tags: ["json", "stdlib"],
    summary: "Encode and decode JSON with the `json` module.",
    output: '{"a":1,"b":[true,2.5]}\n1\n',
    source: `fn main() {
    let text = json_encode({"a": 1, "b": [true, 2.5]})
    print(text)

    let value = json_decode("{\\"n\\": 1}")
    print(value["n"])
}`,
    note: "`json_encode` / `json_decode` are part of the standard library with the `json` feature, which the Playground runtime builds in.",
  },
  {
    id: "regex",
    title: "Regular expressions",
    level: "advanced",
    tags: ["regex", "stdlib"],
    summary: "Match, find, and replace text with the `regex` module.",
    output: 'true\n["7", "42"]\n#/#/#\n',
    source: `fn main() {
    print(regex_match("^a.*z$", "abcxyz"))
    print(regex_find_all("[0-9]+", "id 7 and 42"))
    print(regex_replace("[0-9]+", "2026/01/01", "#"))
}`,
    note: "Functions are `regex_match`, `regex_find`, `regex_find_all`, `regex_replace`; the pattern is always the first argument.",
  },
  {
    id: "tour",
    title: "A tour of Aura",
    level: "advanced",
    tags: ["overview"],
    summary: "A single program touching most of the language.",
    output:
      "Aura: sum 1..5 = 15\nfib(10) = 55\norigin = (0, 0)\ncircle area = 12\nrect area = 12\n[4, 8, 12]\n6\n",
    source: `struct Point { x: int, y: int }

enum Shape {
    Circle(int),
    Rectangle(int, int),
}

fn area(shape) -> int {
    return match shape {
        Circle(r) -> 3 * r * r
        Rectangle(w, h) -> w * h
    }
}

fn fibonacci(n) -> int {
    if n < 2 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    let name = "Aura"
    let mut total = 0
    for i in range(1, 6) {
        total = total + i
    }
    print(f"{name}: sum 1..5 = {total}")
    print(f"fib(10) = {fibonacci(10)}")

    let origin = Point { x: 0, y: 0 }
    print(f"origin = ({origin.x}, {origin.y})")

    print(f"circle area = {area(Circle(2))}")
    print(f"rect area = {area(Rectangle(3, 4))}")

    let xs = [1, 2, 3, 4, 5, 6]
    let evens = xs.filter((x) -> x % 2 == 0)
    let doubled = evens.map((x) -> x * 2)
    print(doubled)
    print(xs |> len)
}`,
    note: "This is the same program as `examples/tour.aura` in the repository.",
  },
];

export function exampleById(id) {
  return examples.find((e) => e.id === id) || null;
}
