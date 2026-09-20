---
layout: default
title: "Expressões"
nav_exclude: true
---

[English](expressions.md) · [Português](expressions.pt_BR.md)

# Expressões

**Status:** Stable (exceto onde rotulado) · **Evidência:** `parser/to_ast.py`
(`_PRECEDENCE`, `parse_expression`, `parse_postfix`, `parse_subscript`),
`transpiler/transformers/expressions.py` (`transform_BinaryOp`,
`transform_UnaryOp`, `transform_RangeExpr`, `transform_ComprehensionExpr`).

Uma expressão produz um valor. A gramática e associatividade estão em
[grammar.md](grammar.md) §6; este documento é a **semântica** de cada forma: o
que ela significa e no que compila. Todo "compila para" abaixo é o Python exato
emitido por `transpiler/transformers/expressions.py`, verificado transpilando o
código-fonte mostrado (*probe*).

---

## 1. Precedência (menor para maior)

Esta tabela é a tabela `_PRECEDENCE` do parser verbatim (`to_ast.py:465-488`).
Número maior liga mais fortemente. Ela corresponde a [grammar.md](grammar.md) §6.1.

| Prec | Operadores | Assoc | Compila para |
|------|-----------|-------|-------------|
| 1 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `&#124;=`, `^=`, `<<=`, `>>=`, `??=`, `&#124;>` | direita (pipe esquerda) | atribuição / `f(x)` |
| 2 | `? :` | direita | `(t if c else f)` |
| 3 | `or` | esquerda | `(a or b)` |
| 4 | `and` | esquerda | `(a and b)` |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`, `is`, `is not` | esquerda | `(a op b)` |
| 6 | `&#124;` | esquerda | `(a &#124; b)` |
| 7 | `^` | esquerda | `(a ^ b)` |
| 8 | `&` | esquerda | `(a & b)` |
| 9 | `<<`, `>>` | esquerda | `(a << b)` |
| 10 | `..`, `..<` | nenhuma | `range(...)` |
| 11 | `??`, `?:` | esquerda / direita* | chamada de helper |
| 12 | `+`, `-` | esquerda | `(a + b)` |
| 13 | `*`, `/`, `%`, `as` | esquerda | `(a * b)` / `T(x)` |
| 14 | `**` | direita | `(a ** b)` |
| 15 | unário `-`, `+`, `~`, `not`, `await`, `...` | prefixo | `(- a)` etc. |
| 16 | call, index, slice, member, safe-nav, struct-init | postfix | `f()`, `a.b`, `a[i]` |

\* `??` é associativo à direita e `?:` é associativo à esquerda — o
`is_left_assoc` do parser retorna `False` para `??` e `True` para `?:`
(`to_ast.py:2654-2655`).
`a ?? b ?? c` → `_aura_null_coalesce(a, _aura_null_coalesce(b, c))`;
`a ?: b ?: c` → `_aura_elvis(_aura_elvis(a, b), c)` (*probe*).

- **Comparação é mais frouxa que o grupo bit a bit e os shifts**, exatamente como
  em Python: `1 & 2 == 2` → `((1 & 2) == 2)` e `1 < 2 | 3` → `(1 < (2 | 3))`
  (`to_ast.py:472-476`, *probe*).
- **Comparação não encadeia**: `a < b < c` → `((a < b) < c)`. Escreva
  `a < b and b < c` (`to_ast.py:2519-2628`, *probe*).
- **Pipe é o operador de expressão mais frouxo**, agrupado com atribuição no
  nível 1 e parseado especialmente para encadear da esquerda para a direita
  (`to_ast.py:2558-2563`). Ele é mais frouxo que `??`: `a |> f ?? b` →
  `_aura_null_coalesce(f, b)(a)` (*probe*).
- **O unário `-` liga mais frouxo que `**`**: `-2 ** 2` → `(- (2 ** 2))` = `-4`,
  enquanto `- 2 * 3` → `((- 2) * 3)` (`to_ast.py:2495-2504`, *probe*).
- **`not` liga mais frouxo que comparação e bit a bit mas mais forte que `and`**:
  `not a in b` → `(not (a in b))`, `not a and b` → `((not a) and b)`
  (`to_ast.py:2487-2494`, *probe*).

---

## 2. Literais

Literais primários são `IntLiteral`, `FloatLiteral`, `StrLiteral`,
`FStringLiteral`, `BoolLiteral`, `NoneLiteral`, e os literais de coleção de
§10. Sua sintaxe concreta está em [lexical-structure.md](lexical-structure.md)
§4 e [grammar.md](grammar.md) §1.4.

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `42` | `42` | `transform_IntLiteral` |
| `3.5` | `3.5` | `transform_FloatLiteral` |
| `"hi"` | `'hi'` | `transform_StrLiteral` |
| `true` / `false` | `True` / `False` | `transform_BoolLiteral` |
| `none` | `None` | `transform_NoneLiteral` |
| `f"x{1+2}"` | `f'x{1 + 2}'` | `transform_FStringLiteral` |

- **`none` é o literal nulo.** `null` → erro de parse apontado "use `none`"
  (`to_ast.py:2686-2688`); `True`/`False`/`None` são grafias reservadas
  (`to_ast.py:536-540`).
- **Campos de f-string** são parseados recursivamente (`to_ast.py:2855-2867`);
  uma conversão `!r`/`!s`/`!a` e um especificador `:format` são preservados
  (`to_ast.py:2884-2918`). Uma expressão de campo inválida é erro de parse.

---

## 3. Identificadores, acesso a membro, indexação, slicing

- `x` — local, parâmetro, membro de módulo ou campo. Uma referência nua a um
  membro de módulo não local resolve para `Module.name` (`transform_Identifier`,
  *probe*: `return f"{owner}.{py_safe_name(node.name)}"`). Um nome que é keyword
  do Python (`raise`, `class`, …) é emitido com um underscore final
  (`py_safe_name`).
- `a.b` — acesso a membro → `a.b` (`transform_MemberExpr`). Nomes de método de
  protocolo (`str`, `eq`, …) mapeiam para seu dunder quando a classe os declara
  (`transform_MemberExpr:552-554`).
- `super` — `super` sem argumentos renderiza como `super()` (`_render_object`)
  então `super.new(...)` / `super.m()` funcionam; `super(args)` renderiza como
  `super().__init__(args)` (`transform_CallExpr:452-454`).
- `a[i]` — indexação → `a[i]` (`transform_IndexExpr`).

```
postfix_op     = "(" , [ arg_list ] , ")"
               | "[" , slice , "]"
               | "?[" , slice , "]"
               | "." , identifier
               | "?." , identifier ;
```

**Slices** são no estilo Python, com qualquer componente omitido
(`to_ast.py:3014-3033`):

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `a[1:4]` | `a[1:4]` | `transform_SliceExpr` |
| `a[::2]` | `a[::2]` | `transform_SliceExpr` |
| `a[i:j:k]` | `a[i:j:k]` | `transform_SliceExpr` |

**Um range usado como índice é um slice, não uma chave**
(`transform_IndexExpr:504-508`, `_range_as_slice`):

| Fonte | Compila para | Nota |
|--------|-------------|------|
| `a[0..<3]` | `a[0:3]` | limite superior exclusivo |
| `a[0..3]` | `a[0:(3) + 1]` | inclusivo → `stop + 1` |
| `a[1..]` | `a[1:]` | fim aberto |
| `a[1..4 step 2]` | `a[1:(4) + 1:2]` | step preservado |

`a[0..]` → `a[0:]` (*probe*). Um limite inicial aberto (`a[..3]`) **não** é
aceito: é erro de parse (*probe*). Um subscrito multidimensional
(`a[0:2, 3]`) é erro de parse — slices são unidimensionais (*probe*).

---

## 4. Chamadas

```
arg_list       = argument , { "," , argument } ;
argument       = expression
               | identifier , "=" , expression        // keyword
               | "*" , expression                     // positional spread
               | "**" , expression                    // keyword spread
               | "..." , expression ;                 // adaptive spread
```

- `f(args)` → `f(args)`; `obj.m(args)` → `obj.m(args)` (`transform_CallExpr`).
  `Klass.m(args)` e um método de módulo/objeto Python emitem o nome verbatim.
- **Argumentos keyword** usam `:=` ou `:` no código-fonte e renderizam o `=` do
  Python (`to_ast.py:3070-3084`): `f(1, b: 2)` → `f(1, b=2)` (*probe*). Um
  argumento posicional após um keyword é erro de parse.
- Uma **expressão geradora como único argumento** é parseada inline
  (`to_ast.py:3090-3112`), então `sum(x for x in xs)` funciona sem parênteses
  extras (*probe*).
- **Spreads**: `*items` → `*items`; `**mapping` → `**mapping`. O marcador é
  colocado em `SpreadExpr.is_dict` (`to_ast.py:3051-3069`).
- **`...value` é adaptativo** (`transform_CallExpr:459-464`): quando é o único
  argumento, a chamada rebaixa para `_aura_call(func, value)`, que desempacota um
  `dict` como argumentos keyword e qualquer outra coisa como argumentos
  posicionais (`transform_CallExpr:386-394` para chamadas de membro). O helper é
  injetado sob demanda.
- **Conveniências de método Aura** se aplicam a chamadas de membro quando o nome
  *não* é um método declarado pelo usuário (`known_member` vence,
  `transform_CallExpr:400-412`):

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `xs.length()` | `len(xs)` | `transform_CallExpr:414-415` |
| `xs.size()` | `len(xs)` | `transform_CallExpr:416-420` |
| `xs.len()` | `len(xs)` | `transform_CallExpr:416-420` |
| `xs.is_empty()` | `(not xs)` | `transform_CallExpr:421-422` |
| `xs.contains(x)` | `(x in xs)` | `transform_CallExpr:423-424` |
| `xs.add(x)` | `xs.append(x)` | `transform_CallExpr:425-428` |
| `s.slice(a)` | `s[a:]` | `transform_CallExpr:429-431` |
| `s.slice(a, b)` | `s[a:b]` | `transform_CallExpr:429-433` |
| `s.char_at(i)` | `s[i]` | `transform_CallExpr:434-435` |

- **Aliases de método de string** mapeiam nomes Aura para nomes Python
  (`METHOD_ALIASES:102-121`): `starts_with`→`startswith`, `ends_with`→`endswith`,
  `to_upper`→`upper`, `to_lower`→`lower`, `trim`→`strip`, `trim_left`→`lstrip`,
  `trim_right`→`rstrip`, `capitalize_words`/`to_title`→`title`,
  `index_of`→`find`, `last_index_of`→`rfind`, `is_alpha`→`isalpha`,
  `is_alphanumeric`→`isalnum`, `is_digit`→`isdigit`, `is_numeric`→`isnumeric`,
  `is_space`→`isspace`, `is_lower`→`islower`, `is_upper`→`isupper`
  (*probe*: `"hello".to_upper()` → `"HELLO"`).
- Um **nome de membro desconhecido** é emitido verbatim (passthrough Python),
  ex.: `xs.append(4)` → `xs.append(4)`, `xs.pop()`, `xs.sort()`
  (`transform_CallExpr:441-444`).

---

## 5. Aritmética: `+ - * / % **`

Aura segue a aritmética **Python**, que difere de JVM/Kof:

| Expressão | Resultado | Por quê | Evidência |
|------------|--------|-----|----------|
| `7 / 2` | `3.5` | `/` é **divisão verdadeira**; nunca trunca | *probe* |
| `-7 % 3` | `2` | `%` segue o **sinal do divisor** (Python); Kof/JVM segue o dividendo | *probe* |
| `7 % -3` | `-2` | mesma regra | *probe* |
| `2 ** 3 ** 2` | `512` | `**` é associativo à direita | *probe* |
| `-2 ** 2` | `-4` | o unário `-` liga mais frouxo que `**` | *probe* |

- **Não há divisão inteira por floor.** `//` sempre inicia um comentário de linha
  ([grammar.md](grammar.md) §1.1); faça um cast para obter um inteiro:
  `int(total / count)`.
- `+ - * / %` renderizam `(a op b)` e `**` renderiza `(a ** b)`
  (`transform_BinaryOp:290-298`).
- `+` sobre duas strings (ou qualquer sequência) é concatenação; `*` com um
  inteiro repete (`"ab" * 2` → `'ab' * 2`, *probe*). Essas são semânticas do
  Python.
- `a + b .. c` → `range((a + b), c + 1)`: `+` liga mais forte que `..`
  (*probe*).

---

## 6. Comparação, igualdade, pertinência, identidade

Todos `< > <= >= == != in not in is is not` ficam na precedência 5 e renderizam
`(a op b)` (`transform_BinaryOp:294-295`).

- **`==`/`!=` são igualdade de valor** via o `==` do Python (semântica Python por
  tipo). **`is`/`is not` são identidade** e renderizam o `is`/`is not` do Python.
- **`x is none` / `x is not none`** é a checagem idiomática de nulo → `(x is None)`
  (*probe*).
- **Identidade contra qualquer outro literal é erro de parse**
  (`to_ast.py:508-518`, `2616-2626`): `x is "a"`, `x is 5`, `x is true`,
  `x is not "a"` todos levantam "'is' compares identity; use '==' (or '!=') to
  compare with a literal" (*probe*). `none` é deliberadamente excluído da
  checagem de literal para que `x is none` seja aceito.
- **`in` / `not in`** renderizam o `in` / `not in` do Python. `3 in [1, 2, 3]` →
  `True` (*probe*).
- **Sem encadeamento de comparação**: `a == b == c` → `((a == b) == c)` (*probe*).

---

## 7. Lógicos: `and or not`

- `and`/`or` renderizam `(a and b)` / `(a or b)` e são **associativos à esquerda**
  (`transform_BinaryOp:293`, `to_ast.py:470-471`).
- Eles usam a **veracidade do Python e retornam um operando**, não um `bool`
  coagido: `1 and 2` → `2`, `0 or "fallback"` → `"fallback"` (*probe*).
- **Curto-circuito**: `and`/`or` não avaliam o operando direito quando o esquerdo
  decide — herdado do Python emitido.
- `not` renderiza `(not a)` (`transform_UnaryOp:323-327`). Ele liga mais frouxo
  que comparação e bit a bit, mais forte que `and` (`to_ast.py:2487-2494`):
  `not 0` → `True`, `not not a` → `(not (not a))` (*probe*).
- **`&&`, `||`, `!` são removidos**: cada um é um erro de parse apontado dizendo
  para usar `and`/`or`/`not` (`to_ast.py:2546-2551`, `2484-2486`, *probe*).

---

## 8. Bit a bit e shifts: `& | ^ ~ << >>`

- `& | ^` renderizam bit a bit entre inteiros; `<<`/`>>` renderizam shifts do
  Python (`transform_BinaryOp:290-298`, *probe*: `x << 2` → `(x << 2)`).
- **`~` é um prefixo unário** em Aura e renderiza `(~ x)`
  (`transform_UnaryOp:323-327`, *probe*). Não há **complemento ausente** — `~x`
  é válido.
- A precedência bit a bit é a do Python: `|` (6) mais frouxo que `^` (7) mais
  frouxo que `&` (8) mais frouxo que shifts (9), todos mais fortes que
  comparação: `x | y & z ^ w` → `(x | ((y & z) ^ w))` (*probe*).
- Não há forma `>>>`; `>>` é o único shift à direita.

---

## 9. Ternário `? :`, Elvis `?:`, coalescência de nulo `??`

| Forma | Significado | Compila para | Evidência |
|------|---------|-------------|----------|
| `c ? t : f` | condicional | `(t if c else f)` | `transform_CondExpr` |
| `a ?: b` | `b` quando `a` é **falsy** (`none`/`false`/`0`/`""`/`[]`/`{}`) | `_aura_elvis(a, b)` | `transform_BinaryOp:262-264` |
| `a ?? b` | `b` apenas quando `a` é `none` | `_aura_null_coalesce(a, b)` | `transform_BinaryOp:259-261` |

- Os helpers `?:` e `??` são injetados sob demanda (`uses_coalesce`,
  `transform_BinaryOp:259-264`). Seus corpos são `value if value is not None
  else default` e `value if value else default` respectivamente (*probe*).
- `a ?? b ?? c` é associativo à direita; `a ?: b ?: c` é associativo à esquerda
  (§1).
- `??=` é a forma de atribuição composta e rebaixa para `x = x if x is not None
  else y` (`x ??= y`, *probe*).
- O ternário liga mais frouxo que tudo até `or`; os ramos parseiam na precedência
  mais baixa (`to_ast.py:2586-2593`): `a ?: b + c` →
  `_aura_elvis(a, (b + c))` (*probe*).

---

## 10. Pipe `|>`

`a |> f` aplica `f` a `a`; `a |> f(b, c)` insere `a` como o **primeiro**
argumento (`transform_PipeExpr:578-591`).

| Fonte | Compila para |
|--------|-------------|
| `a &#124;> f` | `f(a)` |
| `a &#124;> f(b)` | `f(a, b)` |
| `a &#124;> f &#124;> g` | `g(f(a))` |

- Pipe é o **operador de expressão mais frouxo** (nível 1) e encadeia **da
  esquerda para a direita** (`to_ast.py:2558-2563`).
- Quando o lado direito é uma chamada a uma função de coleção da stdlib (`map`,
  `filter`, `reduce`, `take`, `drop`), o transformer injeta o import
  `stdlib.collections` (`transform_PipeExpr`, *probe*). O pipeline canônico está
  em [closures.md](closures.md) §4.

---

## 11. Ranges: `..`, `..<`, `step`, fim aberto

```
range_expr     = coalesce , [ ( ".." | "..<" ) , coalesce , [ "step" , coalesce ] ] ;
```

| Fonte | Compila para | Nota | Evidência |
|--------|-------------|------|----------|
| `1..10` | `range(1, 10 + 1)` | limite superior inclusivo | `transform_RangeExpr` |
| `0..<100` | `range(0, 100)` | exclusivo | `transform_RangeExpr` |
| `0..100 step 5` | `range(0, 100 + 1, 5)` | step | `transform_RangeExpr:643-646` |
| `a..<b step 2` | `range(a, b, 2)` | step com exclusivo | *probe* |
| `1..` | `itertools.count(1)` | fim aberto | `transform_RangeExpr:635-639` |

- Um range avalia para um objeto `range` do Python; `list(1..5)` → `[1, 2, 3, 4, 5]`
  (*probe*).
- **Um range de fim aberto** (`0..`) precisa que o próximo token *não* inicie uma
  expressão final: um fechamento, `{`, `;`, `,`, `)`, `]`, `:`, `=`, `step`, uma
  keyword de statement, EOF, ou um token em uma **linha posterior** o encerram
  (`_range_ends_here`, `to_ast.py:2636-2652`). O transformer injeta
  `import itertools` (`uses_infinite_range`).
- `a .. b + c` → `range(a, (b + c) + 1)`: `+` liga mais forte que `..` (*probe*).
- Usar um range **como índice** é um slice, não `range(...)` — veja §3.

---

## 12. Navegação segura: `?.` e `?[`

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `a?.b` | `(a.b if a is not None else None)` | `transform_SafeNavExpr:561-575` |
| `a?[i]` | `(a[i] if a is not None else None)` | `transform_SafeNavExpr:564-566` |
| `a?.b?.c` | condicional aninhada, um guard por passo | *probe* |
| `a?.b()` | `(a.b if a is not None else None)()` | `transform_CallExpr:386-394` |

- A proteção é contra **`is not None`**, não contra falsidade (*probe*).
- `user?.address?.city` produz `None` quando `user` é `none` (*probe*).
- **Ressalva (Unspecified):** `a?[0..<2]` *não* é convertido em slice — ele emite
  `(a[range(0, 2)] if a is not None else None)`, o que levanta `TypeError` em
  runtime. Apenas o caminho simples `a[0..<2]` (`transform_IndexExpr:504-508`)
  aplica a regra de range-como-slice (*probe*).

---

## 13. Spread: `*`, `**`, `...`

| Contexto | Forma | Compila para | Evidência |
|---------|------|-------------|----------|
| list | `[*a, *b]` | `[*a, *b]` | `transform_ListLiteral` |
| set | `{*a}` | `{*a}` | `transform_SetLiteral:218-225` |
| dict | `{**a, **b}` | `AuraDict({**a, **b})` | `transform_DictLiteral:199-216` |
| call | `f(*a)`, `f(**m)` | `f(*a)`, `f(**m)` | `_render_call_args:479-495` |
| call | `f(...v)` | `_aura_call(f, v)` | `transform_CallExpr:456-464` |

- `*`/`**`/`...` são **marcadores de prefixo**, válidos apenas onde um spread é
  permitido (argumentos de chamada e literais list/set/tuple/dict). Um spread nu
  em posição de valor é erro de parse (`to_ast.py:2505-2515`, *probe*: `[**a]`
  falha).
- `...` dentro de um literal de list ou tuple se comporta como spread posicional
  (`to_ast.py:3170-3171`, `3206-3207`).
- Um nome pontuado e um spread são as únicas alternativas: `1...10` é rejeitado
  com uma dica para usar `..`/`..<` (`to_ast.py:2538-2543`, *probe*).

---

## 14. Comprehensions

```
comprehension  = list_comp | set_comp | dict_comp | generator_expr ;
```

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `[e for p in it]` | `[e for p in it]` | `transform_ComprehensionExpr:764-765` |
| `[e for p in it if c]` | `[e for p in it if c]` | mesma, filtros anexados |
| `{e for p in it}` | `{e for p in it}` | `transform_ComprehensionExpr:766-767` |
| `{k: v for p in it}` | `{k: v for p in it}` | `transform_ComprehensionExpr:736-739` |
| `(e for p in it)` | `(e for p in it)` | `transform_ComprehensionExpr:768-769` |

- **Múltiplas cláusulas `for` e múltiplas `if`** são suportadas e preservam a
  ordem da fonte: `[x*y for x in r for y in s]` → `[(x * y) for x in r for y in s]`
  (*probe*); `[x for x in r if a if b]` encadeia os filtros.
- **Um padrão de tupla de dois nomes sobre um dict insere `.items()`** apenas
  quando o iterável é comprovadamente em forma de dict (um literal dict, uma
  chamada `AuraDict(...)`/`dict(...)`, ou `.items()`/`.keys()`); uma lista de
  pares fica intocada (`_needs_items`, `transform_ComprehensionExpr:748-754`).
- O spread inicial opcional de uma comprehension (`[ *a for x in xs ]`) torna o
  elemento um `SpreadExpr` (`to_ast.py:3170-3171`).
- Comprehensions são **eager**; expressões geradoras são lazy (Python).

---

## 15. Lambdas (expressão)

```
lambda         = ( identifier | "(" , [ param_list ] , ")" ) , "=>" , ( expression | block ) ;
```

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `x => x` | `(lambda x: x)` | `transform_LambdaExpr:662-669` |
| `(a, b) => a + b` | `(lambda a, b: (a + b))` | *probe* |
| `() => 1` | `(lambda : 1)` | *probe* |
| `(x) => { return x }` | um **`def` hoisted**, seu nome referenciado | `_hoist_block_lambda:689-709` |

- Um identificador único antes de `=>` declara uma lambda de um parâmetro
  (`to_ast.py:2719-2722`).
- Uma **lambda com corpo de bloco não pode ser uma `lambda` Python**
  (statements / `return`), então ela é hoisted para uma função real
  `_aura_lambda_N`; locais mutáveis capturados são declarados `nonlocal`
  (`_nonlocal_declaration:711-731`).
- Parâmetros usam a gramática de `def`: defaults, `*args`, `**kwargs`, `*` nu
  (`_render_param:649-660`, `_parse_lambda_params:2942-2993`).
- Veja [closures.md](closures.md) para tipos de função e semântica de captura.

---

## 16. Superfície de expressão de coleções

Literais de coleção e suas operações. A sintaxe concreta está em
[grammar.md](grammar.md) §6.5.

### 16.1 Literais

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `[1, 2, 3]` | `[1, 2, 3]` | `transform_ListLiteral:195-197` |
| `[]` | `[]` | mesma |
| `{"a": 1}` | `AuraDict({'a': 1})` | `transform_DictLiteral:199-216` |
| `{}` | `AuraDict({})` | mesma |
| `{1, 2, 3}` | `{1, 2, 3}` | `transform_SetLiteral:218-225` |
| `()`, `(42,)`, `(1, "a")` | `()`, `(42,)`, `(1, 'a')` | `transform_TupleLiteral:227-231` |
| `Type { x: 1 }` | `Type(**AuraDict({'x': 1}))` | `to_ast.py:3119-3136` |

- Uma **vírgula à direita** é permitida em todo literal: `[1,]` → `[1]`, `{1,}` →
  `{1}` (*probe*).
- `(1)` **não** é uma tupla — é a expressão `1` entre parênteses; uma tupla de um
  elemento precisa da vírgula `(42,)` (`transform_TupleLiteral:229-230`).
- Um **dict** rebaixa através de `AuraDict` (importado sob demanda). Acesso por
  `.name` ou `["key"]` funcionam: `d.name` → `d.name`, `d["age"]` → `d["age"]`
  (*probe*).
- **Struct init** é açúcar para `Type(**AuraDict({...}))` e é reconhecido apenas
  após um identificador **com maiúscula inicial** (`to_ast.py:3119-3136`). Dentro
  de uma condição ou um padrão `case`, `{` abre um bloco/corpo, então o struct
  init é suprimido (`_no_struct_depth`, `_pattern_depth`).
- Um **`{ }` vazio em posição de expressão** parseia como um literal dict vazio,
  não um bloco vazio (veja [statements.md](statements.md) §1).

### 16.2 Indexação, slicing, pertinência, operadores

| Operação | Compila para | Evidência |
|-----------|-------------|----------|
| `xs[i]` (negativo ok) | `xs[i]` | `transform_IndexExpr` |
| `xs[a:b:c]` | `xs[a:b:c]` | `transform_SliceExpr` |
| `x in xs` | `(x in xs)` | `transform_BinaryOp:295` |
| `xs + [6]` | `(xs + [6])` | `transform_BinaryOp` |
| `[0] * 3` | `([0] * 3)` | `transform_BinaryOp` |

Resultados em runtime (*probe*): `xs[-1]` → último elemento, `xs[1:4]` → `[2, 3, 4]`,
`xs[::2]` → elementos alternados, `3 in xs` → `True`, `xs + [6]` →
concatenação, `[0] * 3` → `[0, 0, 0]`.

### 16.3 Conveniências de tamanho, pertinência e mutação

| Fonte | Compila para | Evidência |
|--------|-------------|----------|
| `xs.size()` / `xs.length()` / `xs.len()` | `len(xs)` | tabela §4 |
| `xs.contains(x)` | `(x in xs)` | tabela §4 |
| `xs.is_empty()` | `(not xs)` | tabela §4 |
| `xs.add(x)` | `xs.append(x)` | tabela §4 |
| `xs.append(x)`, `xs.pop()`, `xs.sort()`, `xs.reverse()`, `xs.insert(...)`, `xs.remove(...)`, `xs.index(...)`, `xs.count(...)`, `xs.extend(...)`, `xs.clear()` | emitido verbatim | passthrough Python (`transform_CallExpr:441-444`) |

- Um **método declarado pelo usuário** chamado `size`, `length`, `len`,
  `contains`, `add`, … é um `known_member` e **vence sobre a conveniência**
  (`transform_CallExpr:400-412`).
- `.size()` etc. se aplicam a qualquer objeto com um protocolo `len`/membro
  Python, não só listas (ex.: `dict`, `set`, `str`).

---

## 17. Casts: `as`

`x as T` renderiza `T(x)` (`transform_BinaryOp:284-288`). O lado direito é
parseado como um **tipo** (`to_ast.py:2602-2608`).

| Fonte | Compila para |
|--------|-------------|
| `x as int` | `int(x)` |
| `v as str` | `str(v)` |
| `x as float` | `float(x)` |

`as` fica na precedência 13, com `*`/`/`/`%`: `1 + x as int` → `(1 + int(x))`
e `x as int + 1` → `(int(x) + 1)` (*probe*).

---

## 18. `if`, `match`, `try`, blocos em posição de expressão

Estas construções podem aparecer onde um valor é esperado
(`to_ast.py:2708-2715`, `transform_IfStmt:607-630`, `transform_MatchExpr`,
`transform_TryExpr`, `transform_BlockExpr`).

- `if c { a } else { b }` em posição de expressão → `(a if c else b)`, usando a
  última expressão / `return` de cada ramo.
- `match` / `try` em posição de expressão são **hoisted** para funções auxiliares
  (`_aura_match_N`, `_aura_try_N`); a expressão final do corpo do case vira o
  valor retornado.
- Um `{ … }` nu usado como valor é hoisted para `_aura_block_N()`, cuja última
  expressão é retornada (`transform_BlockExpr:890-931`).
- Atribuição **não** é uma expressão de valor: `x = y` produz um statement, não
  um valor (§1; o parser emite um nó de atribuição, não um `BinaryOp`).

---

## 19. Operações que NÃO são expressões

- `throw`, `return`, `break`, `continue` são **statements**
  ([statements.md](statements.md)).
- `yield` é uma forma de prefixo em nível de statement, não parte de
  `expression`; seu operando parseia na precedência mais baixa, então
  `yield x + 1` produz `x + 1` e `yield a, b` produz a tupla `(a, b)`
  ([grammar.md](grammar.md) §6.1, `to_ast.py:2470-2483`).
- Não há `??=`, `&&`, `||`, `!` como *operadores* (são grafias removidas),
  nem `//` de divisão inteira, nem shift `>>>`.

---

## 20. Resumo das divergências verificadas em relação a Kof/JVM

| Ponto | Aura | Kof/JVM | Evidência |
|-------|------|---------|----------|
| `7 / 2` | `3.5` (divisão verdadeira) | `3` (truncante) | *probe* |
| `-7 % 3` | `2` (sinal do divisor) | `-1` (sinal do dividendo) | *probe* |
| `~x` | complemento válido | `PARSE041` (sem `~`) | *probe* |
| Ternário `? :` | presente | ausente | *probe* |
| `??`, `?:`, `..`, `in` | presente | ausente | `_PRECEDENCE` |
| `is` literal | erro de parse | n/a | *probe* |
