---
layout: default
title: "01 — Instalação"
nav_exclude: true
---

[English](01-installation.md) · [Português](01-installation.pt_BR.md)

# 01 — Instalação

> **Aura 0.2.0a6.** Este capítulo cobre instalar a toolchain, criar um projeto e
> rodá-lo. Os comandos não dependem da versão.

## O que você precisa

Aura roda no CPython, então você precisa de **Python 3.10+**. Todo o resto — o
parser, checker, transpiler, CLI e a biblioteca padrão — vem com o pacote.

## Passo 1 — Instalar o pacote

Aura é publicada no PyPI. A linha atual é uma pré-release, então passe `--pre`:

```bash
python -m pip install --pre aura-language
```

Crie um ambiente isolado primeiro se preferir:

```bash
python -m venv .venv
source .venv/bin/activate        # macOS/Linux
# .venv\Scripts\activate         # Windows
python -m pip install --pre aura-language
```

## Passo 2 — Verificar a instalação

```bash
aura --version
aura --help
```

`aura --help` lista os comandos disponíveis:

| Comando | O que faz |
|---|---|
| `aura run <file.aura>` | transpila e executa um programa |
| `aura check <file.aura>` | checa tipos e regras sem executar |
| `aura repl` | loop interativo de leitura-avaliação-impressão |
| `aura init [name]` | cria um projeto completo com venv |
| `aura transpile <file.aura>` | imprime (ou escreve) o Python gerado |
| `aura format <file.aura>` | formata o código-fonte |
| `aura lint <file.aura>` | reporta warnings de estilo |
| `aura test <dir>` | roda arquivos de teste `.aura` |
| `aura debug <file.aura>` | roda sob o trace debugger |
| `aura lsp` | inicia o language server (stdio) |
| `aura add` / `remove` / `install` / `deps` / `venv` | dependências |
| `aura doctor` | checa o ambiente do projeto |
| `aura version` | mostra ou incrementa a versão |

Os capítulos 02 e 16 usam estes; o resto só precisa de `run`, `check` e `repl`.

## Passo 3 — Criar um projeto com `aura init`

```bash
aura init demo
```

Isso escreve um manifesto e um programa inicial:

```text
✓ Created aura.toml
✓ Created src/main.aura
```

`aura.toml` nomeia o projeto e suas dependências Python:

```toml
[project]
name = "demo"
version = "0.1.0"

[dependencies]
```

`src/main.aura` é o menor programa válido:

```aura
def main() {
  print("Hello from Aura!")
}
```

`aura init --no-venv` pula a criação do `.venv`. Por padrão, `aura init` cria um
`.venv` e instala as dependências declaradas. Para adicionar uma dependência
Python depois, use `aura add <package>`; ele registra o requisito e o instala no
ambiente do projeto.

## Passo 4 — Rodá-lo

```bash
aura run src/main.aura
```

```text
Hello from Aura!
```

## Passo 5 — Checar sem rodar

`aura check` roda os checkers de tipo e de regras e reporta diagnósticos sem
executar o programa. Ele recebe um **arquivo**, não um diretório:

```bash
aura check src/main.aura
```

```text
OK src/main.aura: type check passed
```

Uma checagem que falha sai com código não zero e imprime uma mensagem
posicionada:

```text
src/main.aura:3:3: ERROR [E303]
  Cannot reassign immutable binding 'x'; ...
  hint: write 'let mut x' at its declaration
```

Os códigos de diagnóstico (`E303`, `E310`, `E319`, ...) estão catalogados em
[`../ERRORS.md`](../ERRORS.md).

## Passo 6 — Experimente o REPL

```bash
aura repl
```

O REPL avalia expressões e statements de Aura interativamente. Ele roda as mesmas
checagens de mutabilidade e regras que `aura run`, então um `let` que você
reatribui é um erro lá também.

## Instalar a partir do código-fonte (contribuidores)

```bash
git clone https://github.com/joao/aura-lang.git
cd aura-lang
python -m pip install -e .
```

Uma instalação editável capta mudanças no transpiler sem reinstalar.

## Problemas comuns

| Sintoma | Correção |
|---|---|
| `aura: command not found` | o ambiente não está ativo, ou o diretório de scripts do pip não está no `PATH` |
| `pip` instala uma versão antiga | rode de novo com `--pre` para permitir pré-releases |
| `E310: no main` em `aura run` | o arquivo não tem um `def main()` de nível superior; arquivos de módulo importados não precisam de nenhum |
| `aura check <dir>` dá erro | passe um único arquivo `.aura`; use `aura test <dir>` para um diretório de testes |

## Próximo passo

[Primeiro Programa →](02-first-program.md)
