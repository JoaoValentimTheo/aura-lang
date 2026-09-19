---
layout: default
title: "12 — Tratamento de Erros"
nav_exclude: true
---

[English](12-error-handling.md) · [Português](12-error-handling.pt_BR.md)

# 12 — Tratamento de Erros

> **Meta do capítulo:** sinalizar e recuperar de falhas. Referência:
> [`../language-reference/statements.md`](../language-reference/statements.md)
> §7–10 e [`../language-reference/semantics.md`](../language-reference/semantics.md)
> §5. Exemplo: `../../examples/error_handling.aura`.

## `throw`

`throw` levanta um **valor**. No CPython o valor é levantado:

```aura
def safe_divide(a, b) -> float {
  if b == 0 {
    throw ValueError("division by zero")
  }
  return a / b
}
```

| Forma | Emite |
|---|---|
| `throw ValueError("bad")` | `raise ValueError('bad')` |
| `throw "msg"` | `raise Exception("msg")` — uma string nua é embrulhada |

`UNSPECIFIED`: um valor que não é string nem exceção (`throw 5`) é emitido como
`raise 5` e falha em runtime; não é rejeitado em tempo de check.

## `try` / `catch` / `finally`

```aura
def main() {
  try {
    print(safe_divide(10, 2))
    print(safe_divide(1, 0))
  } catch Error as error {
    print(f"Caught: {error}")
  } finally {
    print("cleanup complete")
  }
}
```

```text
5.0
Caught: division by zero
cleanup complete
```

Um `try` exige **pelo menos um** `catch` ou um `finally`. `finally` sempre roda.

### Grafias de `catch`

Cada grafia tem exatamente um significado:

| Forma | Significado |
|---|---|
| `catch { }` | captura toda exceção, sem binding |
| `catch Type { }` | captura apenas `Type` |
| `catch Type as e { }` | captura apenas `Type`, vincula a `e` |
| `catch as e { }` | captura toda exceção, vincula a `e` |

Um identificador isolado antes de `{` é sempre um **tipo**, nunca um binding; use
`catch Type as name` para vincular. O antigo `catch e { }` ambíguo é erro de
parse.

```aura
def main() {
  try {
    let values = [1]
    print(values[5])
  } catch IndexError {
    print("index out of range")
  }
}
```

`TARGET-SPECIFIC`: cláusulas catch são emitidas na ordem da fonte e o tipo é o
que o CPython resolver para o nome. Aura não reordena subclasses antes dos pais —
liste subclasses primeiro.

## `guard` — valide cedo

`guard cond else { ... }` executa o bloco quando `cond` é falsy, e então continua.
Ele mantém o caminho feliz sem indentação:

```aura
def process(value) {
  guard value > 0 else {
    print(f"invalid value: {value}")
    return
  }
  print(f"processing {value}")
}
```

No nível superior, um `return` nu no corpo do guard sai do programa:

```aura
guard ready else { return }
```

## `try` como expressão

`try { ... } catch ... { ... }` pode produzir um valor, então serve também como
fallback:

```aura
def main() {
  let parsed = try { int("nope") } catch Error as e { 0 }
  print(parsed)          // 0
}
```

## `assert`

```aura
assert 2 + 2 == 4, "math is broken"
```

Quando a condição é falsy, `assert` levanta `AssertionError` com a mensagem dada.
Tanto a condição quanto a mensagem são expressões.

## `with` — gerenciamento de recursos

`with` usa `__enter__`/`__exit__`:

```aura
def main() {
  with open("/tmp/data.txt") as f {
    print(f.read(1))
  }
}
```

`async with` usa `__aenter__`/`__aexit__` e pertence a um `async def`.

## Um exemplo completo

```aura
def safe_divide(a, b) -> float {
  if b == 0 { throw ValueError("division by zero") }
  return a / b
}

def main() {
  try {
    print(safe_divide(10, 2))
    print(safe_divide(1, 0))
  } catch Error as e {
    print(f"Caught: {e}")
  } finally {
    print("done")
  }
}
```

```text
5.0
Caught: division by zero
done
```

## O que você aprendeu

* `throw` levanta um valor; uma string nua é embrulhada.
* `try` exige um `catch` ou um `finally`; quatro grafias de `catch`.
* `guard ... else` para validação antecipada; `return` no nível superior sai.
* `try` como expressão; `assert`; `with`.

## Próximo passo

[Módulos e Imports →](13-modules-and-imports.md)
