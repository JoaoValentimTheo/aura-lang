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
