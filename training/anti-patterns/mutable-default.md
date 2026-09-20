# Anti-Pattern: Mutable Default Arguments

**Severity:** High · **Category:** Functions · **See also:**
[Aura Functions](../../docs/learn/06-functions.md), [Functions Reference](../../docs/language-reference/functions.md)

---

## Problem

A mutable default argument (list, dict, set) is shared across **all calls** to
the function. Every invocation that mutates the default mutates the same object,
producing surprising state leakage between calls.

## The BAD code

```aura
def append_to(item, target = []) {
  target.add(item)
  return target
}

append_to(1)           // [1]
append_to(2)           // [1, 2]  — the default grew across calls!

def update_config(key, value, config = {}) {
  config[key] = value
  return config
}

update_config("a", 1)  // {a: 1}
update_config("b", 2)  // {a: 1, b: 2}  — unwanted carry-over
```

## The GOOD code

```aura
def append_to(item, target: [int] = none) {
  let result = target ?? []
  result.add(item)
  return result
}

append_to(1)           // [1]
append_to(2)           // [2]  — independent each time

def update_config(key, value, config: {str: any} = none) {
  let result = config ?? {}
  result[key] = value
  return result
}

update_config("a", 1)  // {a: 1}
update_config("b", 2)  // {b: 2}  — clean
```

## Key takeaway

Default mutable parameters to `none` and create a fresh object inside the
body. Aura's `??` operator makes this concise.

**Rule:** Every collection parameter that defaults must default to `none`,
never to a literal `[]` or `{}`.
