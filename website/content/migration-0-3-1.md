# Migrating to Aura `0.3.1`

`0.3.1` is the **Keystone** release line (tag `v0.3.1`). Because Keystone changes
observable language behavior, the release version and the language version both
advance to `0.3.1`. This guide lists every change in observable behavior and
what to do about it. Many programs need no changes.

The version-identity model is recorded in
[ADR-0001](https://github.com/JoaoValentimTheo/aura-lang/blob/rewrite/v3-rust/docs/adr/0001-release-vs-language-version.md):
the *release* version identifies a published artifact, the *language* version
identifies the observable language contract, and `language <= release`.

## Summary

| Change | Kind | Who is affected |
|---|---|---|
| Tuple is no longer list sugar; `List`/`Array`/`Tuple`/`Set`/`Map` are distinct identities | intentional break | programs that treated `(a, b)` as a list or compared across kinds |
| Array literals are contextual (`[T; N]`) | new capability | programs that want fixed-length arrays |
| `json_decode_as(text, "T")` → `json_decode_as(text, T)` | API change | programs using the string spelling (it still normalizes) |
| `;` is not a general statement separator (`E1006`) | intentional break | programs using `;` between statements |
| Set uses `{T}` and is limited to key-capable scalar members | new capability | programs wanting broader membership (deferred) |

## Collections have distinct identities

In `0.2`, `Tuple` was absorbed into `List`, so `(1, 2)` was list sugar and there
was no distinct `Array` or `Set`. In `0.3.1` the five kinds are distinct:
`List != Array != Tuple != Set != Map` in equality, and a struct is never equal
to a map.

```aura
# 0.2.1: `t` is a list. 0.3.1: `t` is a Tuple, distinct from a list.
fn main() {
    let t = (1, 2)
    print(t)        # (1, 2)
    print([1, 2])   # [1, 2]
}
```

**Fix:** if your program relied on a tuple behaving as a resizable list, use a
list literal `[a, b]` instead. Tuple literals remain fixed-length, immutable,
and heterogeneous.

## Array literals are contextual (`[T; N]`)

An array type is `[T; N]`, where `N` is a compile-time length. A bracket literal
is a **List** by default and becomes an **Array** only under an expected
`[T; N]` type:

```aura
fn sum3(xs: [int; 3]) -> int {
    return xs[0] + xs[1] + xs[2]
}

fn main() {
    print(sum3([1, 2, 3]))   # [1, 2, 3] realizes as an Array here
    print([1, 2, 3])         # [1, 2, 3] here is a List
}
```

`[1; 2; 3]` is **not** an Array literal; `;` between elements is `E1006`. Array
length and element types must match at the expectation, or the program is
`E3001`. There is no implicit List/Array conversion and no length covariance.

## `json_decode_as` takes a type position

The second argument is now a type, not a string:

```aura
struct Pokemon {
    id: int,
    name: string,
}

fn main() {
    let pokemon = json_decode_as("{\"id\":25,\"name\":\"pikachu\"}", Pokemon)
    print(pokemon.name)   # pikachu
}
```

The older string spelling `json_decode_as(text, "Pokemon")` normalizes to the
same type node, so it still works, but the unquoted type position is the
canonical form. ADR-0001's identity rule applies: the decode restores the
requested collection identity, and a wrong shape is `E4031`.

## `;` is not a general statement separator

Statements are separated by newlines:

```aura
# 0.2.1: accepted. 0.3.1: E1006.
fn main() { let a = 1; let b = 2 }
```

**Fix:** put each statement on its own line (or rely on the zero-width boundary
before `}`). The `;` character is reserved for the `[T; N]` array type and for
string/comment content.

## Set membership is scalar-only

`{T}` is a Set when `T` is a key-capable scalar (`int`, `bool`, `string`).
Broader membership is deferred to a later release.

---

For the full change list see the
[`v0.3.1` release notes](https://github.com/JoaoValentimTheo/aura-lang/blob/rewrite/v3-rust/docs/release-notes/v0.3.1.md)
and [ADR-0005](https://github.com/JoaoValentimTheo/aura-lang/blob/rewrite/v3-rust/docs/adr/0005-keystone-collection-model.md).
