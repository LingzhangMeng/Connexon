# Connexon

<p align="center"><strong>One channel for R and Python.</strong></p>

**License:** MIT · **Status:** Phase 0 · **Tests:** 28 passing

**Connexon** is a Rust runtime and language for bioinformatics, data science and AI.
R packages do the statistics, Python tools do the models, and Connexon owns the data
flow, parallelism and reproducibility between them.

> In a gap junction, two cells each contribute a **connexon**, and together they form
> a channel. Connexon does the same for ecosystems: R brings its statistics, Python
> brings its models, and the language is the channel between them.

---

## Why Connexon?

A modern analysis rarely fits in one language:

- **R** has Bioconductor, formula syntax, mixed models, survival analysis and
  publication-grade figures.
- **Python** has PyTorch, scikit-learn, single-cell foundation models, image analysis
  and the engineering ecosystem needed to serve a model.

Today the boundary is made of ad-hoc CSV files, `reticulate`/`rpy2` objects, copied
environments and two different ideas of what "parallel" means. Connexon replaces that
boundary with a small language and one file contract:

| Problem | Connexon's answer |
|---|---|
| Two runtimes, two package managers | R and Python run as isolated processes; one project, one plan |
| Objects passed across language boundaries | JSON in Phase 0, Arrow IPC for tables and DLPack for tensors in Phase 1–2 |
| Nested parallelism and BLAS oversubscription | One parallelism level; BLAS pinned to one thread per worker |
| Analyses that cannot be reproduced | Explicit scripts, deterministic evaluation, recorded toolchains |
| GPU stacks installed for hardware that cannot use them | A capability probe that refuses unsupported backends |

Connexon is **not** a replacement for R or Python. It is the channel between them.

---

## Status

**Phase 0 vertical slice** — runnable and tested, not yet production-ready.

| Capability | Status |
|---|---|
| `.cnx` lexer, parser, evaluator | ✅ |
| `let`, `fn`, pipes, records, arrays, `if` | ✅ |
| CSV tables, filtering, summaries | ✅ |
| R bridge (`r::call`) | ✅ process-isolated |
| Python bridge (`py::call`) | ✅ process-isolated |
| Hardware probe (`cnx doctor`) | ✅ CPU, memory, GPU, toolchains |
| Install planner (`cnx install --plan-only`) | ✅ plan only, no downloads |
| Arrow data model | ⏳ Phase 1 |
| CUDA / ROCm execution, DLPack | ⏳ Phase 2 |
| AI broker (remote APIs + local models) | ⏳ Phase 2 |
| Package manager and lockfile | ⏳ Phase 1 |

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for exit criteria.

---

## Quick start

### Prerequisites

| Component | Requirement | Notes |
|---|---|---|
| **Rust** | stable toolchain (tested on 1.95.0) | `cargo` must be on `PATH` |
| **R** | 4.0+ (tested on 4.6.1) | `Rscript` on `PATH`; `jsonlite` installed |
| **Python** | 3.10+ (tested on 3.13.13) | `python3` on `PATH` |

R and Python are optional for the core language and tables. They are required only for
`r::` and `py::` calls. The test suite skips bridge tests when an interpreter is absent.

### Build and run

```bash
git clone https://github.com/LingzhangMeng/Connexon.git
cd Connexon

cargo build --workspace -j 6

cargo run -p connexon-cli -- run examples/hello.cnx
cargo run -p connexon-cli -- run examples/functions.cnx
cargo run -p connexon-cli -- run examples/table.cnx
cargo run -p connexon-cli -- run examples/bridge.cnx
cargo run -p connexon-cli -- run examples/pipeline.cnx
```

Expected output from `examples/hello.cnx`:

```text
15
3
1.5811388300841898
```

### Install the `cnx` binary

```bash
cargo install --path crates/connexon-cli
cnx --version
cnx eval "mean([1, 2, 3, 4])"     # 2.5
```

### Or use the Makefile

```bash
make build          # cargo build --workspace -j 6
make test           # cargo test --workspace
make run            # run examples/hello.cnx
make doctor         # probe hardware and print the plan
make install-plan   # write connexon.install-plan.json
make docs           # regenerate every .docx twin (requires pandoc)
```

---

## Five-minute tour

Create `demo.cnx`:

```connexon
# Functions are single expressions.
fn add(a, b) = a + b
fn square(x) = x * x

# Values, arrays and pipes.
let values = [1, 2, 3, 4, 5]
let total = values |> sum()
let average = values |> mean()
let spread = values |> sd()

print(square(add(2, 3)))     # 25
print(total)                 # 15
print(average)               # 3
print(spread)                # 1.5811388300841898

# Records and field access.
let sample = {name: "S01", condition: "A"}
print(sample.condition)      # A

# Tables.
let de = read_csv("examples/data/de_sample.csv")
let hits = de |> filter_where("padj", "<", 0.05) |> length()
print(hits)                  # 4
print(head(de, 3))

# R and Python, each in its own process.
print(r::call("mean", values))          # 3
print(r::call("stats::sd", values))     # 1.58113883008419
print(py::call("statistics", "stdev", values))  # 1.5811388300841898
```

Run it:

```bash
cnx run demo.cnx
```

---

## Language essentials

| Concept | Syntax | Example |
|---|---|---|
| Binding | `let name = expr` | `let x = 42` |
| Function | `fn name(params) = expr` | `fn add(a, b) = a + b` |
| Call | `name(args)` | `mean(values)` |
| Named argument | `name: value` | `head(de, n: 3)` |
| Pipe | `value \|> function(args)` | `de \|> head(3)` |
| Field | `expr.name` | `sample.condition` |
| Array | `[a, b, c]` | `[1, 2, 3]` |
| Record | `{key: value}` | `{gene: "G1", padj: 0.01}` |
| Condition | `if c then a else b` | `if x > 0 then "up" else "down"` |
| R call | `r::call("fn", args...)` | `r::call("mean", values)` |
| Python call | `py::call("module", "fn", args...)` | `py::call("statistics", "stdev", values)` |

**No truthiness.** Conditions must be `bool`; `if 1 then ...` is an error.
**No implicit coercion.** Arithmetic requires numbers; `+` also concatenates strings and arrays.
**No assignment operator.** Binding is only `let`.

Full reference: [`docs/LANGUAGE.md`](docs/LANGUAGE.md) (grammar, precedence, type rules).

---

## CLI reference

| Command | Purpose |
|---|---|
| `cnx run <file.cnx>` | Parse and evaluate a program |
| `cnx eval "<expression>"` | Evaluate one expression |
| `cnx doctor` | Probe CPU, memory, GPU and toolchains; print the install plan |
| `cnx install` | Interactive plan → confirmation → write plan |
| `cnx install --plan-only` | Write `connexon.install-plan.json` and exit |
| `cnx install --yes` | Non-interactive; write the plan without prompting |
| `cnx --help` | Show commands and developer affiliation |
| `cnx --version` | Print the version |

---

## How it works

```text
.cnx source
    │
    ▼
lexer  →  parser  →  AST
    │
    ▼
evaluator
    ├── native builtins (tables, statistics, JSON, CSV; Rayon for large inputs)
    ├── r::  →  Rscript subprocess  →  JSON
    └── py:: →  python3 subprocess  →  JSON
```

Three decisions define the runtime:

1. **R and Python are processes, never embedded.**
   Each call spawns a fresh interpreter, exchanges JSON through temporary files, and
   returns a typed value. This avoids R's single-threaded `longjmp` model, Python's GIL,
   crash propagation and GPL linking. It is also the only portable design on Windows and
   macOS, where `fork` is unavailable or unsafe.
2. **One level of parallelism.**
   Rayon handles large native operations. BLAS is pinned to one thread per worker by
   policy. The runtime never nests worker pools.
3. **The installer is a planner.**
   It probes the machine, proposes components with sizes and licences, and writes a
   plan. It never installs a driver silently and never proposes a backend the hardware
   cannot run.

### Why JSON in Phase 0

JSON is debuggable, dependency-free and easy to inspect. Phase 1 replaces it with
**Arrow IPC** for tables and **DLPack** for GPU tensors, keeping the same process
boundary. The bridge contract is documented in [`docs/DESIGN.md`](docs/DESIGN.md).

---

## Hardware probe and install planner

```bash
cnx doctor
cnx install --plan-only
```

`cnx doctor` reports:

- OS, kernel and architecture
- physical cores, SMT threads, ISA features and NUMA nodes
- total and available memory
- GPUs by vendor, model, architecture and VRAM
- R, Python, CUDA and ROCm toolchains

The planner then chooses a profile (`cpu-only`, `cuda` or `rocm`) and lists components
with version, download size, installed size, licence and reason.

Rules:

- Never install a kernel or display driver automatically.
- Never propose ROCm for an unsupported architecture.
- Prefer user-space installation.
- Print system-level commands separately and require explicit approval.

Example: this development machine has an AMD Radeon PRO WX 3200 (`gfx803`), which modern
ROCm does not support. Connexon detects it, marks `compute: false`, and plans a
**CPU-only** profile instead of installing an unusable GPU stack.

---

## Repository layout

```text
Connexon/
├── .github/
│   └── workflows/ci.yml
├── crates/
│   ├── connexon-core/       values, tables, CSV/JSON, parallel statistics
│   ├── connexon-syntax/     lexer, AST, parser
│   ├── connexon-runtime/    evaluator, builtins, R/Python bridges, system planner
│   └── connexon-cli/        the `cnx` binary
├── examples/
│   ├── hello.cnx
│   ├── functions.cnx
│   ├── table.cnx
│   ├── bridge.cnx
│   ├── pipeline.cnx
│   └── data/de_sample.csv
├── docs/
│   ├── TUTORIAL.md          step-by-step tutorial
│   ├── API_REFERENCE.md     builtins and bridge signatures
│   ├── DEVELOPER_GUIDE.md   architecture, adding builtins, testing
│   ├── FAQ.md               common questions and answers
│   ├── LANGUAGE.md          syntax, grammar, semantics
│   ├── DESIGN.md            architecture and decisions
│   └── ROADMAP.md           phases and exit criteria
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── CITATION.cff
├── AUTHORS.md
├── README.md
└── handoff.md
```

---

## Documentation

| Document | What it covers |
|---|---|
| [`docs/TUTORIAL.md`](docs/TUTORIAL.md) | Install, first program, functions, tables, bridges, exercises |
| [`docs/API_REFERENCE.md`](docs/API_REFERENCE.md) | Every builtin and bridge function with signatures and examples |
| [`docs/DEVELOPER_GUIDE.md`](docs/DEVELOPER_GUIDE.md) | Architecture, adding builtins, testing, release checklist |
| [`docs/FAQ.md`](docs/FAQ.md) | Common questions, design rationale, troubleshooting |
| [`docs/LANGUAGE.md`](docs/LANGUAGE.md) | Formal EBNF, precedence, type rules, scoping |
| [`docs/DESIGN.md`](docs/DESIGN.md) | Architecture decisions and data contracts |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Phase 1–3 plan and risks |

Every `.md` document has a `.docx` twin with Times New Roman, 仿宋 for Chinese and red
monospace code, generated by `make docs`.

---

## Examples

| File | Demonstrates |
|---|---|
| `examples/hello.cnx` | Values, arrays, `sum`, `mean`, `sd` |
| `examples/functions.cnx` | User functions, recursion-ready expressions, `if` |
| `examples/table.cnx` | `read_csv`, `count_where`, `head`, `select` |
| `examples/bridge.cnx` | Live R and Python calls on the same vector |
| `examples/pipeline.cnx` | End-to-end CSV → filter → R mean → Python SD |

Run any of them with:

```bash
cnx run examples/hello.cnx
```

---

## Testing

```bash
cargo test --workspace
# or
make test
```

The suite contains **28 tests**:

| Suite | Tests | Coverage |
|---|---:|---|
| `connexon-core` | 4 | JSON round trip, table detection, CSV inference, statistics |
| `connexon-syntax` | 7 | Lexer, precedence, pipes, namespaces, functions, records, errors |
| `connexon-runtime` | 10 | Evaluation, functions, pipes, tables, R bridge, Python bridge, system probe |
| `connexon-cli` | 7 | `run`, `eval`, table, bridge, `doctor`, install plan, error exit |

Formatting and lints:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Continuous integration runs the same three commands on Ubuntu.

---

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Rscript was not found` | R is not installed or not on `PATH` | Install R; ensure `Rscript --version` works |
| `python3 was not found` | Python is not on `PATH` | Install Python 3.10+; ensure `python3 --version` works |
| `bridge error: ...` | The R or Python function raised | Read the message; test the call in R/Python directly |
| `unknown function` | Typo or a Phase 1 feature | Check [`docs/API_REFERENCE.md`](docs/API_REFERENCE.md) |
| `unknown column` | Column name mismatch | Column names are case-sensitive; use `head(de)` to inspect |
| `cannot compare bool and number` | `a < b < c` | Write `a < b && b < c` |
| `if` condition must be `bool` | No truthiness | Use an explicit comparison |
| `division by zero` | `/` or `%` with zero | Guard the denominator |
| R/Python call is slow | One process per call | Batch in Phase 1; for now, keep calls coarse-grained |
| GPU not used | No supported backend | Run `cnx doctor`; `gfx803` is unsupported by modern ROCm |
| `Permission denied (publickey)` | Deploy key missing write access | Enable **Allow write access** on GitHub, or use a PAT |

More answers in [`docs/FAQ.md`](docs/FAQ.md).

---

## Roadmap

| Phase | Focus | Exit criterion |
|---|---|---|
| **0** (done) | Parser, evaluator, bridges, probe, installer plan, CLI | 28 tests pass; examples run |
| **1** | Arrow tables, Parquet, typed stubs, lockfile, LSP | Real bulk RNA-seq pipeline with DESeq2 and scikit-learn |
| **2** | DAG scheduler, native CPU kernels, CUDA/ROCm, DLPack, AI broker | Same pipeline on CPU or GPU with one flag |
| **3** | Static types, modules, pattern matching, registry, platform packages | Third-party packages and programs |

Details: [`docs/ROADMAP.md`](docs/ROADMAP.md).

---

## Contributing

Contributions are welcome. Before opening a pull request:

1. Build and test: `make build && make test`.
2. Format and lint: `make fmt && make clippy`.
3. Add a test for every new builtin, operator or bridge behaviour.
4. Update the relevant document in `docs/` and run `make docs`.
5. Keep R and Python process-isolated; do not embed interpreters.

The developer guide explains how to add a builtin, a bridge function and a test:
[`docs/DEVELOPER_GUIDE.md`](docs/DEVELOPER_GUIDE.md).

---

## Citation

If you use Connexon in academic work, cite it with the metadata in
[`CITATION.cff`](CITATION.cff):

```bibtex
@software{connexon2026,
  title        = {Connexon: a Rust runtime and language for R, Python and AI},
  author       = {Meng, Lingzhang},
  year         = {2026},
  version      = {0.1.0},
  license      = {MIT},
  url          = {https://github.com/LingzhangMeng/Connexon}
}
```

---

## Developer

**Prof. Dr. rer. nat. Lingzhang Meng** (孟令章 教授)

Research Center for Medical Big Data and Artificial Intelligence,
The People's Hospital of Guangxi Zhuang Autonomous Region
(Guangxi Academy of Medical Sciences)

广西壮族自治区人民医院（广西医学科学院）医学大数据与人工智能研究中心

孟令章教授[广西壮族自治区人民医院（广西医学科学院）,医学大数据与人工智能研究中心]

- GitHub: [@LingzhangMeng](https://github.com/LingzhangMeng)
- Email: lzmeng@gxams.org.cn

---

## Naming

| Element | Value |
|---|---|
| Language / runtime | Connexon |
| CLI | `cnx` |
| File extension | `.cnx` |
| Config / lockfile | `connexon.toml` / `connexon.lock` |
| Chinese name | 连接子 (liánjiēzǐ) |
| Tagline | *Two cells. One channel.* |

---

## License

MIT. See [LICENSE](LICENSE).
