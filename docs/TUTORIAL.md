# Connexon tutorial

This tutorial takes you from a clean checkout to a working R + Python pipeline.
Every command and output below was executed on Linux (Ubuntu 26.04) with **Rust 1.95.0**,
**R 4.6.1** and **Python 3.13.13**.

---

## Before you start

| Requirement | Check | Needed for |
|---|---|---|
| Rust stable | `cargo --version` | Building the runtime |
| R 4.0+ with `jsonlite` | `Rscript --version` | `r::` calls |
| Python 3.10+ | `python3 --version` | `py::` calls |

R and Python are optional for the core language and tables. They are required only when
you call `r::` or `py::`.

---

## Lesson 0 — build and install

```bash
git clone https://github.com/LingzhangMeng/Connexon.git
cd Connexon

cargo build --workspace -j 6
cargo install --path crates/connexon-cli

cnx --version
```

Expected:

```text
cnx 0.1.0
```

> Run the tutorial commands **from the repository root**, because the examples use
> relative paths such as `examples/data/de_sample.csv`.

---

## Lesson 1 — values and output

Create `hello.cnx`:

```connexon
print("Hello, Connexon")
print(42)
print(true)
print(null)
```

Run:

```bash
cnx run hello.cnx
```

Output:

```text
Hello, Connexon
42
true
null
```

Connexon has six value types: `null`, `bool`, `number`, `string`, `array`, `record`,
plus `table` when you read a CSV. Numbers are 64-bit floats.

---

## Lesson 2 — bindings and arithmetic

```connexon
let x = 10
let y = 4

print(x + y)       # 14
print(x - y)       # 6
print(x * y)       # 40
print(x / y)       # 2.5
print(x % y)       # 2
print(x > y)       # true
```

`let` introduces a **global** binding. Re-declaring a name overwrites it.

Arithmetic requires numbers. `+` also concatenates two strings or two arrays:

```connexon
print("R" + " + " + "Python")   # R + Python
print([1, 2] + [3, 4])          # [1, 2, 3, 4]
```

---

## Lesson 3 — functions and recursion

Functions are single expressions:

```connexon
fn add(a, b) = a + b
fn square(x) = x * x
fn factorial(n) = if n <= 1 then 1 else n * factorial(n - 1)

print(add(2, 3))        # 5
print(square(add(2, 3))) # 25
print(factorial(5))     # 120
```

Functions see globals and their own parameters. They are not closures. Recursion and
mutual recursion work because functions are looked up at call time:

```connexon
fn is_even(n) = if n == 0 then true else is_odd(n - 1)
fn is_odd(n) = if n == 0 then false else is_even(n - 1)
print(is_even(10))      # true
```

---

## Lesson 4 — arrays, records and fields

```connexon
let values = [1, 2, 3]
let sample = {name: "S01", condition: "A", reads: 437000}

print(values)            # [1, 2, 3]
print(sample.name)       # S01
print(sample.reads)      # 437000
print(length(values))    # 3
print(range(4))          # [0, 1, 2, 3]
```

Records use `{key: value}`. Keys may be identifiers or strings. Field access is
`record.key`.

---

## Lesson 5 — pipes

The pipe injects the left value as the **first argument**:

```connexon
let values = [1, 2, 3, 4, 5]

print(values |> sum())    # 15
print(values |> mean())   # 3
print(values |> sd())     # 1.5811388300841898
```

Chaining evaluates left to right:

```connexon
print(values |> sum() |> sqrt())   # 3.872983346207417
```

`|>` binds looser than `||`. The right side stops at one postfix expression, so
`a |> f() || b` is a syntax error; write `(a |> f()) || b`.

---

## Lesson 6 — if is an expression

```connexon
let x = 7
print(if x > 0 then "positive" else "non-positive")   # positive
```

Conditions must be `bool`. There is no truthiness:

```text
cnx: type error: expected bool, got number
```

That error is produced by `if 1 then 2 else 3`.

---

## Lesson 7 — tables from CSV

The example file has six genes and seven columns.

```connexon
let de = read_csv("examples/data/de_sample.csv")

print(length(de))                                     # 6 rows
print(head(de, 3))                                    # first three rows
print(count_where(de, "padj", "<", 0.05))             # 4
print(select(de, "gene", "log2FoldChange"))           # two columns
```

`head` accepts a named argument:

```connexon
print(head(de, n: 3))
```

Column access returns an array:

```connexon
let p = column(de, "padj")
print(length(p))                                      # 6
```

Column names are case-sensitive. Use `head(de)` to inspect them.

---

## Lesson 8 — filter and write

```connexon
let de = read_csv("examples/data/de_sample.csv")
let hits = filter_where(de, "padj", "<", 0.05)

print(length(hits))          # 4
write_csv(hits, "hits.csv")
```

`filter_where` keeps rows whose column satisfies the comparison. The operator must be
one of `==`, `!=`, `<`, `<=`, `>`, `>=`.

Your first row-boundary bug will probably be `a < b < c`. It parses, but it compares a
boolean with a number and fails. Write `a < b && b < c`.

---

## Lesson 9 — calling R

```connexon
print(r::available())                          # true
print(r::version())                            # Rscript (R) version 4.6.1 (2026-06-24)

print(r::call("mean", [1, 2, 3, 4, 5]))        # 3
print(r::call("stats::sd", [1, 2, 3, 4, 5]))   # 1.58113883008419
print(r::call("stats::median", [1, 2, 3, 4, 5]))  # 3
```

The first argument to `r::call` is the function name. Use `package::function` to reach a
package namespace. Each call spawns a fresh `Rscript` process; an R error becomes a
Connexon error with the R message.

---

## Lesson 10 — calling Python

```connexon
print(py::available())                              # true
print(py::version())                                # Python 3.13.13

print(py::call("statistics", "stdev", [1, 2, 3, 4, 5]))
# 1.5811388300841898

print(py::call("builtins", "sum", [1, 2, 3, 4, 5]))  # 15
```

The first argument is the module, the second is the function. Dotted attributes work.
Keyword arguments are not supported yet.

---

## Lesson 11 — JSON

```connexon
let payload = {gene: "G1", padj: 0.01, hits: [1, 2]}
let text = to_json(payload)
print(text)

let back = from_json(text)
print(back.gene)     # G1
print(back.hits)     # [1, 2]
```

Exact JSON output:

```text
{"gene":"G1","hits":[1.0,2.0],"padj":0.01}
```

Two details:

- Record keys are emitted in sorted order.
- Integer-valued numbers serialize as `1.0` because the language has one numeric type.

`from_json` turns an array of objects into a table.

---

## Lesson 12 — a complete pipeline

`examples/pipeline.cnx`:

```connexon
# CSV -> filter -> R mean -> Python standard deviation
let de = read_csv("examples/data/de_sample.csv")
let hits = de |> filter_where("padj", "<", 0.05)
let n = length(hits)
let log2fc = column(hits, "log2FoldChange")

let r_mean = r::call("mean", log2fc)
let py_sd = py::call("statistics", "stdev", log2fc)

print(n)
print(r_mean)
print(py_sd)
```

Run:

```bash
cnx run examples/pipeline.cnx
```

Verified output:

```text
4
0.775
1.840335114410779
```

The table is filtered natively, the mean is computed by R, and the standard deviation by
Python — each in its own process, with JSON at the boundary.

---

## Lesson 13 — errors

| Program | Error |
|---|---|
| `1 +` | `cnx: expected expression, found end of input at line 1, column 4` |
| `if 1 then 2 else 3` | `cnx: type error: expected bool, got number` |
| `unknown(1)` | ``cnx: unknown function `unknown` `` |
| `1 / 0` | `cnx: division by zero` |
| `count_where(de, "nope", "<", 1)` | ``cnx: unknown column `nope` `` |

Errors go to stderr and the process exits non-zero, so pipelines fail loudly.

---

## Exercises

1. Write `fn cube(x) = x * x * x` and print `cube(3)`.
   *Answer: 27.*
2. Count genes with `padj < 0.001` in the sample table.
   *Answer: 3.*
3. Compute the median in R and the mean in Python for `[1, 2, 3, 4, 5]`.
   *Answer: `r::call("stats::median", values)` → 3; `py::call("statistics", "mean", values)` → 3.*
4. Write the filtered table to `hits.csv`.
   *Answer: `write_csv(filter_where(de, "padj", "<", 0.05), "hits.csv")`.*
5. Explain why `1 < 2 < 3` fails, then fix it.
   *Answer: it becomes `(1 < 2) < 3`, i.e. `true < 3`; write `1 < 2 && 2 < 3`.*

---

## Where next

- [`API_REFERENCE.md`](API_REFERENCE.md) — every builtin and bridge signature.
- [`LANGUAGE.md`](LANGUAGE.md) — formal grammar, precedence and type rules.
- [`DEVELOPER_GUIDE.md`](DEVELOPER_GUIDE.md) — add a builtin or a bridge function.
- [`FAQ.md`](FAQ.md) — design questions and troubleshooting.
- [`ROADMAP.md`](ROADMAP.md) — Arrow, GPU and AI support.
