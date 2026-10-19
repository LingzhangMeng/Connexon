# Connexon FAQ

## General

### Is Connexon a replacement for R or Python?

No. It is a channel between them. R keeps Bioconductor and its statistical libraries;
Python keeps PyTorch and its engineering ecosystem. Connexon owns the program that
composes them.

### Why Rust?

A small static binary, predictable memory behaviour, mature parallelism (`rayon`),
Arrow support (`arrow-rs`) and a credible path to a typed front end. It also gives one
portable process model on Linux, macOS and Windows.

### Does Connexon embed R or Python?

No. Every `r::` and `py::` call spawns a fresh `Rscript` or `python3` process and
exchanges JSON through temporary files. This avoids R's single-threaded `longjmp` model,
Python's GIL, crash propagation and GPL linking.

### Is Connexon statically typed?

Not in Phase 0. Values are dynamically typed and every operator checks at run time.
Gradual typing is on the roadmap for Phase 3.

### Does it run on Windows and macOS?

The design is portable: subprocesses plus JSON, no `fork`. Phase 0 is tested on Linux;
CI runs on Ubuntu. macOS and Windows builds are planned for the platform matrix.

---

## Usage

### How do I run a program?

```bash
cnx run program.cnx
```

Run from the repository root if the program uses relative paths such as
`examples/data/de_sample.csv`.

### How do I evaluate a single expression?

```bash
cnx eval "mean([1, 2, 3, 4])"
```

### How do I inspect a table?

```bash
cnx eval 'head(read_csv("examples/data/de_sample.csv"), 3)'
```

Or inside a program:

```connexon
let de = read_csv("examples/data/de_sample.csv")
print(head(de, 3))
print(select(de, "gene", "padj"))
```

### How do I write a table?

```connexon
write_csv(hits, "hits.csv")
```

### Why does `a < b < c` fail?

It parses as `(a < b) < c`. The left side is a boolean, and ordering a boolean against a
number is a type error. Write:

```connexon
a < b && b < c
```

### Why is `a |> f() || b` a syntax error?

The right side of `|>` is one postfix expression, so the pipe consumes `f()` and stops.
The `|| b` is then left without a statement separator. Write:

```connexon
(a |> f()) || b
```

### Why is there no truthiness?

`if 1 then ...` is rejected. R and Python disagree about which values are true; requiring
`bool` removes a class of silent bugs. Write an explicit comparison.

### Why are numbers all floats?

Phase 0 has one numeric type (`f64`), which keeps arithmetic and JSON simple. JSON output
therefore shows integer-valued numbers as `1.0`. A distinction between integers and
floats is a candidate for the typed phase.

### How do I get the number of rows?

`length(table)` returns the row count. `length(array)` returns the element count.

### How do I count rows matching a condition?

```connexon
count_where(de, "padj", "<", 0.05)
```

The operator must be one of `==`, `!=`, `<`, `<=`, `>`, `>=`, and it must be a string.

---

## R and Python

### Can I call DESeq2?

Not as a first-class typed API yet. You can call simple R functions with
`r::call("package::function", args...)`, but complex objects such as `DESeqDataSet`
cannot cross the JSON boundary. Typed wrappers are Phase 1 work.

### Can I use scikit-learn?

You can call plain functions and simple estimators through `py::call`, but returning a
fitted estimator object is not supported yet. Typed estimator wrappers and Arrow/DLPack
handoff are Phase 1–2.

### Can I use PyTorch or a GPU?

Not in Phase 0. GPU execution, DLPack handoff and the device scheduler are Phase 2.
`cnx doctor` already detects GPUs and plans the correct backend.

### Why does my GPU show `compute: false`?

The GPU is not usable for modern compute. A common case is an AMD `gfx803` card, which
ROCm no longer supports. Connexon refuses to plan a stack that cannot run.

### Why is each R/Python call slow?

Each call starts a new interpreter. That is the price of isolation and portability.
Phase 1 adds Arrow IPC and batched calls; keep calls coarse-grained until then.

### Can I pass a DataFrame or a tensor?

Not yet. Scalars, vectors, records and arrays of records are supported. DataFrames,
AnnData objects and tensors need the Phase 1–2 adapters.

### Why is `r::call("mean", x)` not returning what I expect?

Check the value being passed. A Connexon array becomes an R vector; a record becomes a
named list. For package functions, use `"package::function"`. Test the same call in R
first to isolate the problem.

---

## Project

### Why is the CLI called `cnx`?

`cnx` is short to type and avoids colliding with an existing `connexon` command. The
project, language and file extension remain `Connexon` / `.cnx`.

### How do I cite Connexon?

Use `CITATION.cff` or the BibTeX entry in `README.md`.

### How do I contribute?

Read `docs/DEVELOPER_GUIDE.md`, run `make test`, and add a test for every behaviour
change. Keep R and Python process-isolated.

### Where are the design decisions recorded?

`docs/DESIGN.md` records the architecture decisions: process isolation, JSON now and
Arrow later, one parallelism level, the installer as a planner, platform artifacts, and
the AI broker.

### What is planned next?

Arrow IPC and Parquet, typed wrappers for DESeq2 and scikit-learn, a lockfile, GPU
execution with CUDA/ROCm, and an AI broker. See `docs/ROADMAP.md`.

### Is the license MIT?

Yes. See `LICENSE`.
