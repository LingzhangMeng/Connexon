# Connexon API reference

This document lists everything the Phase 0 runtime exposes. All examples use the
repository root as the working directory.

Notation:

- `->` shows the return type.
- `...` means one or more positional arguments.
- `name: value` is a named argument, accepted only where stated.

---

## Values

| Type | Literal / constructor | Notes |
|---|---|---|
| `null` | `null` | Only equal to itself |
| `bool` | `true`, `false` | Required by conditions |
| `number` | `42`, `3.14`, `1.2e-28` | 64-bit float |
| `string` | `"GENE0001"` | Escapes `\n \t \r \\ \"` |
| `array` | `[1, 2, 3]` | Heterogeneous allowed |
| `record` | `{gene: "G1", padj: 0.01}` | Keys are identifiers or strings |
| `table` | `read_csv(...)` | Named columns, row storage |

---

## Output and types

### `print(value) -> null`

Prints a value to standard output. Tables print an aligned preview of up to 10 rows.

```connexon
print([1, 2, 3])        # [1, 2, 3]
print({a: 1})           # {a: 1}
print(null)             # null
```

### `type_of(value) -> string`

Returns `null`, `bool`, `number`, `string`, `array`, `record` or `table`.

```connexon
print(type_of([1, 2]))  # array
```

### `length(value) -> number`

Returns the number of elements, characters, record fields or table rows.

```connexon
length([1, 2, 3])       # 3
length("abc")           # 3
length({a: 1, b: 2})    # 2
length(read_csv("examples/data/de_sample.csv"))  # 6
```

Errors on `null`, `bool` and `number`.

---

## Tables

### `read_csv(path) -> table`

Reads a UTF-8 CSV with a header row. Cells are inferred as `null` (empty or `NA`),
`bool`, `number` or `string`.

```connexon
let de = read_csv("examples/data/de_sample.csv")
```

### `write_csv(table, path) -> null`

Writes a header and all rows. `null` becomes an empty field.

```connexon
write_csv(hits, "hits.csv")
```

### `head(table, n = 5) -> table`

First `n` rows. `n` may be positional or named.

```connexon
head(de)
head(de, 3)
head(de, n: 3)
```

### `tail(table, n = 5) -> table`

Last `n` rows. Same argument rules as `head`.

### `select(table, names...) -> table`

Projects columns in the requested order. Every name must be a string and must exist.

```connexon
select(de, "gene", "log2FoldChange")
```

### `column(table, name) -> array`

Returns one column as an array, preserving row order.

```connexon
column(de, "padj")
```

### `filter_where(table, column, op, value) -> table`

Keeps rows where `row[column] op value` is true.

- `column` is a string.
- `op` is one of `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Comparison is numeric or string-to-string; other combinations are false.

```connexon
filter_where(de, "padj", "<", 0.05)
de |> filter_where("padj", "<", 0.05)
```

### `count_where(table, column, op, value) -> number`

Same predicate as `filter_where`, returning the count.

```connexon
count_where(de, "padj", "<", 0.05)   # 4
```

---

## Statistics and math

### `mean(values) -> number`

Arithmetic mean of an array of numbers. Errors on an empty array.

### `sum(values) -> number`

Sum of an array of numbers.

### `sd(values) -> number`

**Sample** standard deviation (denominator `n - 1`). Needs at least two values.

### `min(values) -> number` / `max(values) -> number`

Minimum and maximum of an array of numbers. Errors on an empty array.

### `abs(x) -> number`

Absolute value.

### `sqrt(x) -> number`

Square root. Negative input yields `NaN`.

### `round(x) -> number`

Rounds half away from zero, matching Rust's `f64::round`.

### `floor(x) -> number` / `ceil(x) -> number`

Round down / up.

```connexon
mean([1, 2, 3, 4])   # 2.5
sum([1, 2, 3, 4])    # 10
sd([1, 2, 3, 4])     # 1.2909944487358056
min([1, 2, 3, 4])    # 1
max([1, 2, 3, 4])    # 4
```

> Large arrays are processed with Rayon. Small arrays stay serial to avoid scheduling
> overhead.

---

## JSON and arrays

### `to_json(value) -> string`

Serializes a value. Records emit sorted keys. Integer-valued numbers appear as `1.0`
because the numeric type is `f64`.

```connexon
to_json({gene: "G1", hits: [1, 2], padj: 0.01})
# {"gene":"G1","hits":[1.0,2.0],"padj":0.01}
```

### `from_json(text) -> value`

Parses JSON. An array of objects with identical keys becomes a **table**; other arrays
become arrays; objects become records.

```connexon
let back = from_json("{\"a\": 1, \"b\": [2, 3]}")
print(back.a)   # 1
```

### `range(n) -> array`

Returns `[0, 1, ..., n - 1]`. Negative `n` yields an empty array.

```connexon
range(4)   # [0, 1, 2, 3]
```

---

## R bridge

### `r::call(function, args...) -> value`

- `function` is a string. Use `package::function` for a package namespace.
- Remaining arguments are passed positionally.
- Each call spawns a new `Rscript` process.
- An R error becomes a Connexon error with the R message.

```connexon
r::call("mean", [1, 2, 3, 4, 5])          # 3
r::call("stats::sd", [1, 2, 3, 4, 5])     # 1.58113883008419
r::call("stats::median", [1, 2, 3, 4, 5]) # 3
r::call("paste", "a", "b")                 # a b
```

### `r::available() -> bool`

Whether `Rscript` is on `PATH`.

### `r::version() -> string | null`

First line of `Rscript --version`, or `null`.

### Conversion rules

| Connexon | R |
|---|---|
| number | length-one numeric vector |
| array of numbers/strings | atomic vector |
| record | named list |
| array of records | list of named lists |
| table | array of records (list of lists) |

Results are converted back through JSON. An array of objects becomes a table. Complex R
objects such as `DESeqDataSet` are not supported in Phase 0.

---

## Python bridge

### `py::call(module, function, args...) -> value`

- `module` is a string, for example `"statistics"` or `"builtins"`.
- `function` may contain dots: `"linear_model.LogisticRegression"`.
- Remaining arguments are passed positionally.
- Each call spawns a new `python3` process.
- Exceptions become Connexon errors with the Python message.

```connexon
py::call("statistics", "stdev", [1, 2, 3, 4, 5])   # 1.5811388300841898
py::call("statistics", "mean", [1, 2, 3, 4, 5])    # 3
py::call("builtins", "sum", [1, 2, 3, 4, 5])       # 15
```

### `py::available() -> bool`

Whether `python3` is on `PATH`.

### `py::version() -> string | null`

First line of `python3 --version`, or `null`.

### Conversion rules

Arguments are JSON-encoded. Results are JSON-decoded; values that are not JSON native are
stringified by Python's `default=str`. Keyword arguments are not supported yet.

---

## Operators

| Precedence | Operators | Operand rules |
|---|---|---|
| 1 (tightest) | call, field | — |
| 2 | unary `-`, `!`, `not` | number / bool |
| 3 | `*`, `/`, `%` | numbers |
| 4 | `+`, `-` | `+`: numbers, strings or arrays; `-`: numbers |
| 5 | `==`, `!=`, `<`, `<=`, `>`, `>=` | equality: any; ordering: numbers or strings |
| 6 | `&&`, `and` | bool, short-circuit |
| 7 | `\|\|`, `or` | bool, short-circuit |
| 8 (loosest) | `\|>` | injects left as first argument |

`/` and `%` by zero are errors. Comparisons do not chain mathematically.

---

## Error messages

| Message | Meaning |
|---|---|
| `expected expression, found ...` | Syntax error at a line and column |
| `type error: expected bool, got number` | Condition or logical operand is not bool |
| `unknown variable ...` | `let` was not executed before use |
| `unknown function ...` | Not a builtin, user function or bridge function |
| `unknown column ...` | Table column name mismatch |
| `division by zero` | `/` or `%` with zero |
| `bridge error: ...` | The R or Python call raised |
| `Rscript was not found on PATH` | R is not installed or not visible |
| `python3 was not found on PATH` | Python is not installed or not visible |

---

## Phase 0 limits

- No keyword arguments for `r::call` or `py::call`.
- No complex object handoff (DESeqDataSet, DataFrame, PyTorch tensors).
- No vectorised table expressions; use the builtins.
- No package/import management in the language.
- No GPU tensors or AI calls.
- Tables are row-oriented in memory; Arrow is Phase 1.
