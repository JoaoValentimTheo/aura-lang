# Anti-Pattern: Stringly-Typed Code

**Severity:** High · **Category:** Types · **See also:**
[Types](../../docs/language-reference/types.md), [Enums](../../docs/language-reference/classes.md#8-enums), [Pattern Matching](../../docs/learn/11-pattern-matching.md)

---

## Problem

Using raw strings like `"admin"` or `"pending"` instead of proper types.
The compiler cannot catch misspellings, and every comparison is a fragile
equality check against a magic string.

## The BAD code

```aura
def get_role_color(role: str) -> str {
  if role == "admin" { return "red" }
  if role == "editor" { return "blue" }
  if role == "viewer" { return "green" }
  return "gray"
}

// Wrong case — silently returns "gray":
let color = get_role_color("Admin")

def process_order(status: str) {
  if status == "pending" { /* ... */ }
  if status == "shipped" { /* ... */ }
}
```

## The GOOD code

```aura
enum Role { Admin, Editor, Viewer }
enum OrderStatus { Pending, Shipped, Delivered }

def get_role_color(role: Role) -> str {
  return match role {
    Role.Admin => "red",
    Role.Editor => "blue",
    Role.Viewer => "green",
  }
}

let color = get_role_color(Role.Admin)    // compile-time guarantee

def process_order(status: OrderStatus) {
  return match status {
    OrderStatus.Pending => "processing",
    OrderStatus.Shipped => "tracking",
    OrderStatus.Delivered => "completed",
  }
}
```

## Key takeaway

Use **enums** for finite value sets and **classes** for structured data.
Pattern matching with `match` gives exhaustive, compile-time-checked branching.

| Instead of | Use |
|-----------|-----|
| `"admin"` / `"editor"` | `enum Role { Admin, Editor }` |
| `{"type": "circle", "r": 5}` | `class Circle { public let radius: float }` |

**Rule:** Never use a raw string as a type proxy. Use `enum` or `class`.
