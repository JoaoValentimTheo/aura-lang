# Collections

## Lists

`[a, b, c]` constructs a list. Lists are ordered, mutable, and have reference
semantics.

```aura
let mut xs = [1, 2, 3]
xs[0] = 10          # index assignment
xs.push(4)          # append
let last = xs.pop() # remove and return the last element
```

Mutating a list — through index assignment, a mutating builtin such as `push`
or `pop`, or a `mut self` method — requires the binding to be `mut`. This is
the general mutation-capability rule, not a special case for `push`:

```aura
let ys = [1, 2]
ys.push(3)          # E2001: ys is immutable
```

* Indexing uses an integer; negative indices count from the end. Out of range is
  `E4019`.
* Equality is length- and element-wise.
* Ordering is not defined.

## Maps

`{k: v, ...}` constructs a map, ordered by key. Keys must be key-capable:
`string`, `int`, or `bool`, or a union of these.

```aura
let mut m = {"a": 1, "b": 2}
m["c"] = 3
print(m.get("missing"))   # none
print(m.has("a"))         # true

let counts = {1: "one", 2: "two"}
print(counts[2])          # two
```

* `m[k]` is `E2003` when the key is absent; `m.get(k)` returns `none`.
* `m[k] = v` inserts or replaces.
* Iteration yields keys in ascending order.
* Equality compares key sets and values. `1` and `"1"` are different keys.
* A key type of `float`, `none`, a list, or a map is `E3001`.

Generic maps use the same syntax with a type parameter for the key:

```aura
type Map<K, V> = {K: V}

let scores: Map<int, float> = {1: 9.5, 2: 8.75}
print(scores[1])          # 9.5
```

The key parameter must be key-capable when the alias is instantiated, so
`Map<float, int>` is rejected.

## The empty map

`{:}` is the empty-map literal. `{}` is an empty *block*, not an empty map; the
two are distinct.

```aura
let mut empty = {:}
empty["k"] = 1
```

## Parenthesized lists

Aura has no distinct tuple type. `(a, b)` is list sugar: it constructs a list of
its elements, indistinguishable from `[a, b]`.

## Pipelines over collections

The transformation methods return new lists, so they chain naturally.

```aura
let xs = [5, 3, 8, 1]
let ascending = xs.sort()
let doubled = xs.map((x) -> x * 2)
let evens = xs.filter((x) -> x % 2 == 0)
let total = xs.reduce((a, b) -> a + b, 0)
```

Available list methods: `len`, `push`, `pop`, `first`, `last`, `join`,
`contains`, `sort`, `reverse`, `map`, `filter`, `reduce`. Free functions `map`,
`filter`, `reduce`, `sort`, `reverse`, `sum`, `enumerate`, and `zip` take the
list as an argument, which is convenient in a pipeline.

## Map entries

`m.items()` yields the map's entries as two-element lists `[key, value]` in
ascending key order. It is eager: the returned list is a snapshot, so mutating
the map afterwards does not change it.

```aura
let users = {1: "Ada", 2: "Alan"}
for [id, name] in users.items() {
    print(id)
    print(name)
}
```

`for k in users` still iterates keys only — `items()` makes the pair explicit.

## Comprehensions

A comprehension builds a collection in one expression. There is exactly one
generator clause and an optional filter:

```aura
let xs = [1, 2, 3, 4]

let doubled = [x * 2 for x in xs]
let evens = [x for x in xs if x % 2 == 0]
let squares = {x: x * x for x in xs}
```

They are eager, the iterable is evaluated once, the pattern is assertive like
`for`, and bindings do not leak. A comprehension is an expression, so it can be
used anywhere a value is expected. It is not a replacement for statement
loops: there is no `break`/`continue`, no `yield`, and no multiple generators.

```aura
let names = [
    name
    for [id, name] in users.items()
    if id > 1
]
```

## Literal types

A literal infers a union of its parts: `[1, "x"]` is `[int | string]`, and
`{1: "a", "1": "b"}` is `{int | string: string}`. When the expected type is
known, every element is checked against it, so `let xs: [int] = [1, "x"]` is a
type error rather than a silent surprise.
