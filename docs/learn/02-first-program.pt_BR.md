---
layout: default
title: "02 — Primeiro Programa"
nav_exclude: true
---

[English](02-first-program.md) | [Português](02-first-program.pt_BR.md)

# 02 — Primeiro Programa

> **Meta do capítulo:** escrever, rodar e entender o menor programa Aura. Exemplo:
> [`../../examples/hello.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/hello.aura).

## O programa

Crie um arquivo `hello.aura`:

```aura
def main() {
  print("Hello, Aura!")
}
```

Rode-o:

```bash
aura run hello.aura
```

Saída:

```text
Hello, Aura!
```

Esse é um programa Aura completo. Cada peça importa:

| Peça | Significado |
|---|---|
| `def` | declara uma função; é a **única** keyword de função |
| `main` | o ponto de entrada — o runtime o chama para você |
| `()` | `main` não recebe parâmetros |
| `{ ... }` | o corpo da função, um bloco de statements |
| `print(...)` | escreve seu argumento na saída padrão |
| `"..."` | um literal de string |

## Por que não há uma chamada `main()`

Em muitas linguagens você escreve `main()` ou `if __name__ == "__main__":`. Em
Aura, o runtime invoca `main` por conta própria. Escrever uma chamada `main()` à
direita é desnecessário, e **omitir `main` em um arquivo executado é o erro
`E310`**.

Um arquivo importado como módulo (capítulo 13) não precisa de `main` algum.

## Statements e quebras de linha

Statements são separados pela gramática de Aura, e um `;` de terminação é
**opcional**. Quebras de linha não têm significado sintático, então estes dois
são o mesmo programa:

```aura
def main() {
  print("one")
  print("two")
}
```

```aura
def main() { print("one"); print("two") }
```

Escolha um estilo e seja consistente; `aura format` normaliza para indentação de
dois espaços e chaves na mesma linha.

## Comentários

```aura
// a line comment

/* a block comment
   spanning lines */
```

`//` sempre inicia um comentário de linha — Aura deliberadamente **não** tem o
operador `//` de divisão inteira. Use `int(a / b)` para um quociente inteiro.

## Imprimindo mais

`print` aceita vários argumentos (eles são impressos separados por espaço):

```aura
def main() {
  let name = "world"
  print("Hello,", name)
  print(f"Hello, {name}!")
}
```

```text
Hello, world
Hello, world!
```

A forma `f"..."` é uma **f-string**: `{...}` embute uma expressão. F-strings
também suportam especificadores de formato e conversões `!r`/`!s`/`!a`:

```aura
def main() {
  let pi = 3.14159
  print(f"pi is about {pi:.2f}")
}
```

```text
pi is about 3.14
```

## Um primeiro programa um pouco maior

```aura
def greet(name, punctuation = "!") -> str {
  return "Hello, " + name + punctuation
}

def main() {
  print(greet("world"))
  print(greet("Aura", "?"))
}
```

```text
Hello, world!
Hello, Aura?
```

Duas ideias novas aparecem aqui: uma segunda função e um **parâmetro default**
(`punctuation = "!"`, usado quando o chamador o omite). `-> str` é a anotação de
tipo de retorno. Funções são o capítulo 06; por ora, note que o código Aura se lê
de cima para baixo e toda construção é explícita.

## Vendo o Python gerado

Como Aura transpila, você pode inspecionar o target:

```bash
aura transpile hello.aura
```

A saída é Python comum. É uma forma útil de confirmar o que uma construção faz —
e um lembrete de que o runtime é CPython.

## O que você aprendeu

* `def main() { ... }` é o ponto de entrada; o runtime o chama.
* `print` escreve saída; f-strings interpolam expressões.
* Statements terminam em fronteiras da gramática; `;` é opcional.
* `//` e `/* ... */` são comentários.
* `aura run` executa, `aura transpile` mostra o Python.

## Próximo passo

[Noções Básicas →](03-language-basics.md)
