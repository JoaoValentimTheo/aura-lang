---
layout: default
title: "16 — CLI e Tooling"
nav_exclude: true
---

[English](16-cli-and-tooling.md) | [Português](16-cli-and-tooling.pt_BR.md)

# 16 — CLI e Tooling

> **Meta do capítulo:** usar a CLI `aura` para rodar, checar, formatar, testar e
> inspecionar programas. Referência:
> [`../language-reference/semantics.md`](../language-reference/semantics.md) §8.

## `aura run`

Transpila e executa um programa:

```bash
aura run src/main.aura
aura run src/main.aura --            # pass program arguments
aura run src/main.aura -v            # also print the generated Python
```

`main` recebe os argumentos em seu único parâmetro `args`. Se ele retorna um
`int`, isso vira o código de saída do processo.

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

```bash
aura run app.aura one two
```

```text
2 argument(s)
```

## `aura check`

Checa tipos e regras **sem** rodar. Ele recebe um **arquivo**:

```bash
aura check src/main.aura
```

```text
OK src/main.aura: type check passed
```

Diagnósticos carregam um código estável (`E303`, `E310`, `E319`, ...) e uma dica
opcional. `aura check` reporta erros de tipo `E1xx` que `aura run` não reporta —
o caminho de run executa apenas as checagens de mutabilidade e regras. Os códigos
estão catalogados em [`../ERRORS.md`](../ERRORS.md).

## `aura repl`

Um prompt interativo que avalia Aura e roda as mesmas checagens de
mutabilidade/regras que `aura run`:

```bash
aura repl
```

Útil para experimentar expressões e checar a que uma construção avalia.

## `aura init`

Cria um projeto inicial:

```bash
aura init demo           # writes aura.toml and src/main.aura
aura init demo --venv    # also creates .venv and installs dependencies
```

## `aura transpile`

Imprime (ou escreve) o Python gerado:

```bash
aura transpile src/main.aura              # to stdout
aura transpile src/main.aura -o out.py    # to a file
aura transpile src/main.aura -v           # also show the AST
```

Esta é a forma mais clara de confirmar o que uma construção faz em runtime.

## `aura format`

Normaliza o código-fonte para indentação de dois espaços, chaves na mesma linha e
um espaço ao redor de operadores binários:

```bash
aura format src/main.aura              # print formatted source
aura format src/main.aura -i           # rewrite in place
aura format src/main.aura -o clean.aura
```

O conteúdo de literais de string nunca é reescrito.

## `aura lint`

Reporta warnings de estilo e convenção sem falhar um build:

```bash
aura lint src/main.aura
```

```text
✓ src/main.aura: no style issues
```

## `aura test`

Roda arquivos de teste `.aura` em um diretório. Cada arquivo é executado com
`--no-main` (o arquivo se autogoverna), então um arquivo de teste não precisa de
`main`:

```bash
aura test tests/
aura test tests/ -v
aura test tests/ -p "*_test.aura"
```

Um arquivo de teste usa `assert` e roda suas próprias checagens:

```aura
def test_addition() {
  assert 2 + 2 == 4
}

def test_strings() {
  assert "a" + "b" == "ab"
}

test_addition()
test_strings()
print("all tests passed")
```

```text
all tests passed

1/1 passed, 0 failed (0.09s)
```

## `aura debug`

Roda um arquivo sob o trace debugger leve:

```bash
aura debug src/main.aura
aura debug src/main.aura --trace
```

## `aura lsp`

Inicia o language server sobre stdio para integração com editores:

```bash
aura lsp
```

## Dependências e ambiente

| Comando | O que faz |
|---|---|
| `aura add <pkg>` | adiciona uma dependência Python e a instala |
| `aura remove <pkg>` | remove uma dependência declarada |
| `aura install` | instala dependências de `aura.toml` |
| `aura deps` | lista dependências declaradas (`--lock` escreve `aura.lock`) |
| `aura venv init` | cria `.venv` e instala dependências |
| `aura doctor` | checa o ambiente do projeto |
| `aura version` | mostra ou incrementa a versão |

`aura add` escreve em `aura.toml` e instala no ambiente do projeto; uma
dependência declarada fica então alcançável com o prefixo `py.` (capítulo 14).

## Um loop típico

```bash
aura init demo
aura run src/main.aura
aura check src/main.aura
aura format src/main.aura -i
aura lint src/main.aura
aura test tests/
```

## O que você aprendeu

* `run` (com args e `-v`), `check` (tipos + regras), `repl`.
* `init`, `transpile`, `format`, `lint`.
* `test` (arquivos que se autogovernam), `debug`, `lsp`.
* Comandos de dependência e ambiente; um loop típico de edição.
