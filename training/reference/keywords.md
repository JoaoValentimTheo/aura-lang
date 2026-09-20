# Aura Keywords — Quick Reference

## All 53 Reserved Keywords

```
abstract  and       as        assert    async     await     break
case      catch     class     const     continue  def       else
enum      export    false     finally   fn        for       from
guard     if        import    in        is        let       loop
match     module    new       none      not       or        private
protected public    return    self      spawn     static    super
throw     trait     true      try       type      unless    until
volatile  while     with      yield
```

> Keywords are **case-sensitive**: `if` is reserved, `If` is an identifier.
> `fn` is reserved and rejected with: *"use `def` instead"*.

## Declarations

| Keyword | Usage | Example |
|---------|-------|---------|
| `let` | Immutable binding | `let x = 1` |
| `let mut` | Mutable binding | `let mut count = 0` |
| `const` | Constant (must init) | `const MAX = 100` |
| `def` | Function | `def add(a, b) -> int` |
| `class` | Class | `class User { ... }` |
| `trait` | Trait (interface) | `trait Printable { ... }` |
| `enum` | Enumeration | `enum Color { RED, GREEN }` |
| `type` | Type alias | `type ID = int` |
| `module` | Namespace | `module M { ... }` |

## Control Flow

| Keyword | Usage | Example |
|---------|-------|---------|
| `if` | Conditional | `if x > 0 { ... }` |
| `else` | Alternative | `} else { ... }` |
| `unless` | Inverted if | `unless done { ... }` |
| `guard` | Early exit | `guard x != none else { return }` |
| `match` | Pattern matching | `match val { case ... }` |
| `case` | Match arm | `case 0 { ... }` |
| `for` | Loop / iteration | `for x in xs { ... }` |
| `while` | Loop (pre-test) | `while i < 10 { ... }` |
| `until` | Inverted while | `until done { ... }` |
| `loop` | Infinite loop | `loop { ... }` |
| `break` | Exit loop | `break` / `break label` |
| `continue` | Skip iteration | `continue` / `continue label` |
| `step` | Range/for step | `for i in 0..10 step 2` |
| `in` | Iteration / membership | `for x in xs` / `x in xs` |

## Error Handling

| Keyword | Usage | Example |
|---------|-------|---------|
| `try` | Try block | `try { ... }` |
| `catch` | Catch exceptions | `catch Error as e { ... }` |
| `finally` | Cleanup block | `finally { ... }` |
| `throw` | Raise exception | `throw ValueError("bad")` |
| `return` | Return from function | `return 42` / `return a, b` |
| `assert` | Assertion | `assert x > 0, "must be positive"` |

## OOP

| Keyword | Usage | Example |
|---------|-------|---------|
| `new` | Constructor call | `User("Alice")` |
| `self` | Instance reference | `self.name` |
| `super` | Parent reference | `super.new(...)` |
| `abstract` | Abstract class | `abstract class Base { ... }` |
| `static` | Static member | `static def create() { ... }` |
| `enum` | Enumeration | `enum Direction { UP, DOWN }` |

## Visibility

| Keyword | Usage | Example |
|---------|-------|---------|
| `public` | Public (default) | `public let x = 1` |
| `private` | File-private | `private def helper() { ... }` |
| `protected` | Class-protected | `protected let data = []` |
| `export` | Export from module | `export def api() { ... }` |

## Async

| Keyword | Usage | Example |
|---------|-------|---------|
| `async` | Async function | `async def fetch() { ... }` |
| `await` | Await promise | `let data = await fetch()` |
| `spawn` | Launch thread/task | `spawn(worker, arg)` |

## Other

| Keyword | Usage | Example |
|---------|-------|---------|
| `import` | Import module | `import stdlib.math` |
| `from` | Named import | `from lib import greet` |
| `with` | Resource management | `with open("f") as fh { ... }` |
| `yield` | Generator yield | `yield value` |
| `volatile` | (Reserved, unused) | — |

## Literal Keywords

| Keyword | Type | Usage |
|---------|------|-------|
| `true` | `bool` | Boolean true |
| `false` | `bool` | Boolean false |
| `none` | `none` | Null / absence |

> `null`, `True`, `False`, `None` are **not valid** — use `none`/`true`/`false`.
