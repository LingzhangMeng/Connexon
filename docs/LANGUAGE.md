# The Connexon language

Connexon is a small, expression-oriented language for composing data analysis. It is statically parseable, dynamically typed in Phase 0, and designed to become gradually typed.

## Lexical structure

- **Comments** start with `#` and run to the end of the line.
- **Identifiers** match `[A-Za-z_][A-Za-z0-9_]*`.
- **Numbers** are 64-bit floats, including exponent notation (`1.5e-9`).
- **Strings** use double quotes and support `\n`, `\t`, `\r`, `\\` and `\"`.
- **Statements** are separated by newlines or `;`.

## Formal grammar (EBNF)

Notation: `{ x }` is zero or more, `[ x ]` is optional, and `|` separates alternatives.

```ebnf
program        = { separator } , [ statement , { separator , statement } ] , { separator } ;
separator      = newline | ";" ;
statement      = let_stmt | fn_def | expression ;
let_stmt       = "let" , identifier , "=" , expression ;
fn_def         = "fn" , identifier , "(" , [ identifier , { "," , identifier } ] , ")" , "=" , expression ;
expression     = pipe ;
pipe           = or , { "|>" , postfix } ;
or             = and , { ( "||" | "or" ) , and } ;
and            = comparison , { ( "&&" | "and" ) , comparison } ;
comparison     = additive , { ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) , additive } ;
additive       = multiplicative , { ( "+" | "-" ) , multiplicative } ;
multiplicative = unary , { ( "*" | "/" | "%" ) , unary } ;
unary          = ( "-" | "!" | "not" ) , unary | postfix ;
postfix        = primary , { call | field } ;
call           = "(" , [ argument , { "," , argument } , [ "," ] ] , ")" ;
field          = "." , identifier ;
argument       = expression | identifier , ":" , expression ;
primary        = number | string | "true" | "false" | "null"
               | identifier , "::" , identifier , call
               | identifier , [ call ]
               | "(" , expression , ")"
               | "[" , [ expression , { "," , expression } , [ "," ] ] , "]"
               | "{" , [ field_pair , { "," , field_pair } , [ "," ] ] , "}"
               | "if" , expression , "then" , expression , "else" , expression ;
field_pair     = ( identifier | string ) , ":" , expression ;
```

Layout consequences:

- A statement must be followed by a newline or `;` unless it is the last statement.
- `let` and `fn` are statements, so they cannot appear inside an expression or after `|>`.
- A trailing comma is accepted in arrays, records and call arguments.

## Values

| Type | Examples |
|---|---|
| null | `null` |
| bool | `true`, `false` |
| number | `42`, `3.14`, `1.2e-28` |
| string | `"GENE0001"` |
| array | `[1, 2, 3]` |
| record | `{gene: "G1", padj: 0.01}` |
| table | the result of `read_csv(...)` |

Tables are first-class: CSV columns are typed by inference (`null`, bool, number, string), and a table prints as an aligned preview.

## Statements

### Let binding

```connexon
let values = [1, 2, 3, 4, 5]
let average = mean(values)
```

### Function definition

```connexon
fn add(a, b) = a + b
fn square(x) = x * x
let result = square(add(2, 3))
```

Functions are expressions with lexical scope over globals. They take positional arguments only.

## Expressions

### Arithmetic and comparison

```connexon
1 + 2 * 3          # 7
(1 + 2) * 3        # 9
10 / 4             # 2.5
10 % 3             # 1
2 > 1 && 3 <= 4    # true
```

`+` also concatenates two strings or two arrays.

### Operator precedence

From tightest to loosest:

| Level | Operators | Associativity |
|---|---|---|
| 1 | call `(...)`, field `.name` | left |
| 2 | unary `-`, `!`, `not` | right |
| 3 | `*`, `/`, `%` | left |
| 4 | `+`, `-` | left |
| 5 | `==`, `!=`, `<`, `<=`, `>`, `>=` | left (see note) |
| 6 | `&&`, `and` | left, short-circuit |
| 7 | `||`, `or` | left, short-circuit |
| 8 | `|>` | left; the right side is one postfix expression |

Notes:

- Comparisons chain syntactically, but `a < b < c` compares a boolean with a number and fails at run time. Write `a < b && b < c`.
- `|>` is lower than `||`, so `a || b |> f()` pipes the whole logical expression. The right side stops at the first postfix expression, so `a |> f() || b` is a syntax error; write `(a |> f()) || b`.
- `if ... then ... else ...` is a primary expression: its branches extend as far right as possible. Use parentheses when mixing it with operators.

### Pipe

The pipe injects the left value as the first argument of the call on the right:

```connexon
[1, 2, 3] |> length()        # 3
[1, 2, 3] |> mean()          # 2
```

### If

`if` is an expression:

```connexon
let label = if result > 20 then "large" else "small"
```

### Records and fields

```connexon
let sample = {name: "S01", condition: "A", reads: 437000}
sample.condition          # "A"
```

A table also supports field access, which returns a column as an array:

```connexon
de.padj                   # array of adjusted p-values
```

## Semantics and type rules

The language is dynamically typed in Phase 0. Every operator checks its operands at run time and returns a typed error rather than coercing silently.

| Construct | Accepted operands | Result |
|---|---|---|
| `a + b` | number + number, string + string, array + array | same type |
| `a - b`, `a * b`, `a / b`, `a % b` | number + number | number |
| `a / b`, `a % b` with `b == 0` | — | error |
| `a < b`, `a <= b`, `a > b`, `a >= b` | number + number, string + string | bool |
| `a == b`, `a != b` | any two values, compared structurally | bool |
| `a && b`, `a || b`, `!a` | bool + bool | bool, short-circuit |
| `if c then a else b` | `c` must be bool | value of the taken branch |
| `-a` | number | number |
| `x |> f(args)` | `f` is a function or builtin | `f(x, args)` |

There is **no truthiness**: numbers, strings and arrays are not accepted as conditions. `null` only participates in `==` and `!=`.

`+` on tables is not defined; use `select`, `filter_where` or a bridge.

## Scoping and evaluation order

- `let` introduces a **global** binding. Redeclaring a name overwrites it. There are no block scopes yet.
- Function parameters shadow globals for the duration of the call.
- Functions see **globals and their parameters**, not the caller's locals. They are not closures.
- Recursion and mutual recursion work: functions are looked up at call time.
- Statements evaluate top to bottom; arguments and array/record elements evaluate left to right.
- Named arguments are accepted by builtins (for example `head(de, n: 3)`); user functions take positional arguments only.
- A user function's body is a single expression; there is no `return` and no block body.

## What the parser rejects

- `let` or `fn` inside an expression, or after `|>`
- Two statements on one line without `;`
- Assignment other than `let` (`x = 1` is only valid after `let`)
- A namespace call without parentheses (`r::mean`)
- A record key that is not an identifier or a string
- Raw newlines inside a string
- Characters outside the token set (for example `@`, `$`, `\`)

## Builtins

| Builtin | Description |
|---|---|
| `print(value)` | Print a value; tables print as a preview |
| `type_of(value)` | Type name |
| `length(value)` | Array, string, record or table length |
| `read_csv(path)` | Read a CSV file into a table |
| `write_csv(table, path)` | Write a table to CSV |
| `head(table, n = 5)` | First `n` rows |
| `tail(table, n = 5)` | Last `n` rows |
| `select(table, names...)` | Project columns |
| `column(table, name)` | One column as an array |
| `filter_where(table, col, op, value)` | Filter rows |
| `count_where(table, col, op, value)` | Count matching rows |
| `mean(values)` / `sum(values)` / `sd(values)` | Statistics |
| `min(values)` / `max(values)` | Extremes |
| `abs(x)` / `sqrt(x)` / `round(x)` / `floor(x)` / `ceil(x)` | Numeric helpers |
| `to_json(value)` / `from_json(text)` | JSON conversion |
| `range(n)` | `[0, 1, ..., n-1]` |

Operators accepted by `filter_where` and `count_where`: `==`, `!=`, `<`, `<=`, `>`, `>=`.

## Calling R

```connexon
r::call("mean", [1, 2, 3, 4])          # base R
r::call("stats::sd", [1, 2, 3, 4])     # package namespace
r::available()                          # bool
r::version()                            # string or null
```

Arguments are converted from JSON. Numeric and string vectors become R vectors; records become named lists. Results are converted back to Connexon values; an array of records becomes a table.

The call is executed by a new `Rscript` process. An R error becomes a Connexon error with the R message.

## Calling Python

```connexon
py::call("statistics", "stdev", [1, 2, 3, 4])
py::call("builtins", "sum", [1, 2, 3])
py::available()
py::version()
```

The first argument is the module, the second is the function. Dotted attributes work (`"linear_model.LogisticRegression"`), and keyword arguments are not yet supported. Results are converted from JSON; non-JSON objects are stringified.

## Errors

- Syntax errors report line and column.
- Type errors name the expected and actual type.
- Unknown columns, variables, functions and namespaces are explicit.
- Bridge failures include the interpreter's message.
- The CLI exits non-zero on any error.

## What is intentionally missing in Phase 0

- Static types and type inference
- Modules and imports
- Pattern matching
- Closures over caller locals
- `for` / `while`
- Vectorised table expressions
- Arrow-backed zero-copy tables
- GPU tensors and DLPack
- AI provider calls

Those are sequenced in `ROADMAP.md`.
