# Collections

## Lists

`[a, b, c]` constructs a list. Lists are ordered, mutable, and have reference
semantics.

```aura
let xs = [1, 2, 3]
xs[0] = 10          # index assignment
xs.push(4)          # append
let last = xs.pop() # remove and return the last element
```

* Indexing uses an integer; negative indices count from the end. Out of range is
  `E4019`.
* Equality is length- and element-wise.
* Ordering is not defined.

## Maps

`{k: v, ...}` constructs a string-keyed map, ordered by key.

```aura
let m = {"a": 1, "b": 2}
m["c"] = 3
print(m.get("missing"))   # none
print(m.has("a"))         # true
```

* `m[k]` is `E2003` when the key is absent; `m.get(k)` returns `none`.
* `m[k] = v` inserts or replaces.
* Iteration yields keys in ascending order.
* Equality compares key sets and values.

## The empty map

`{:}` is the empty-map literal. `{}` is an empty *block*, not an empty map; the
two are distinct.

```aura
let empty = {:}
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
