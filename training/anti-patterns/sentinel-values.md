# Anti-Pattern: Sentinel Values (`-1`, `""`, `0` for "not found")

**Severity:** High · **Category:** Types · **See also:**
[Types](../../docs/language-reference/types.md), [Type System](../../docs/language-reference/type-system.md), [Variables and Types](../../docs/learn/04-variables-and-types.md)

---

## Problem

Using magic numbers or strings like `-1`, `0`, or `""` to represent "absent"
or "not found" is ambiguous. Callers cannot tell whether `-1` is a valid
value or an error signal. It mixes data with control flow.

## The BAD code

```aura
def find_user(users: [User], id: int) -> int {
  for i in 0..<users.length() {
    if users[i].id == id { return i }
  }
  return -1     // sentinel: is -1 a valid index? who knows
}

// Callers must remember to check — easy to forget:
let idx = find_user(users, 42)
if idx != -1 { print(users[idx]) }
```

## The GOOD code

```aura
def find_user(users: [User], id: int) -> User? {
  for user in users {
    if user.id == id { return user }
  }
  return none    // explicit absence — no ambiguity
}

// Callers use ? and ?? — the language enforces awareness:
let user = find_user(users, 42)
print(user?.name ?? "not found")
```

## Key takeaway

Use `T?` (optional) to represent absence. Aura's `none`, `??`, and `?.` make
optional handling concise and explicit.

| Instead of | Use |
|-----------|-----|
| `return -1` | `return none` with `T?` return type |
| `if x == -1 { ... }` | `x?.method() ?? fallback` |

**Rule:** Never use a magic value to mean "absent". Use `T?` instead.
