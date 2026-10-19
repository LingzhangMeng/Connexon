# Connexon developer guide

This guide is for contributors who want to extend the runtime, add a builtin, add a
bridge, or work on the installer.

---

## Prerequisites

| Tool | Version used | Purpose |
|---|---|---|
| Rust | 1.95.0 stable | Build and test |
| R | 4.6.1 with `jsonlite` | Exercise `r::` calls |
| Python | 3.13.13 | Exercise `py::` calls |
| `cargo fmt`, `cargo clippy` | bundled with Rust | Formatting and lints |

---

## Repository layout

```text
crates/
├── connexon-core/       Value, Table, CSV/JSON, statistics
├── connexon-syntax/     lexer, AST, parser
├── connexon-runtime/    evaluator, builtins, bridges, installer planner
└── connexon-cli/        the cnx binary and CLI integration tests
```

Dependency direction is one-way:

```text
connexon-cli → connexon-runtime → connexon-syntax
                         └──────→ connexon-core
```

`connexon-syntax` does not depend on the runtime. Keep the parser free of evaluation
concerns.

---

## Build, test and lint

```bash
make build       # cargo build --workspace -j 6
make test        # cargo test --workspace
make fmt         # cargo fmt --all --check
make clippy      # cargo clippy --workspace --all-targets -- -D warnings
```

Run a single crate:

```bash
cargo test -p connexon-syntax
cargo test -p connexon-runtime
cargo test -p connexon-cli --test cli
```

The bridge tests call `r_available()` / `py_available()` and return early when an
interpreter is absent. Do not turn a missing optional dependency into a hard failure.

---

## Data model

`connexon-core` defines:

```rust
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Table(Table),
}
```

`Table` is row-oriented in Phase 0:

```rust
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}
```

Phase 1 replaces the internals with Arrow record batches. Keep new code behind the
`Value`/`Table` surface so that change stays local.

---

## How evaluation works

1. `connexon_syntax::parse` produces a `Program` of `Stmt`.
2. `Runtime::run` evaluates statements against global and local scopes.
3. `Runtime::eval` handles expressions; `eval_call` dispatches to:
   - a user function,
   - a native builtin,
   - `r::` or `py::` bridge functions.
4. Builtins receive `&[(Option<String>, Value)]` so they can support named arguments.

---

## Add a builtin

Example: add `sign(x)`.

### 1. Add the dispatch arm

In `crates/connexon-runtime/src/lib.rs`, inside `call_builtin`:

```rust
"sign" => self.builtin_sign(args),
```

### 2. Implement the method

```rust
fn builtin_sign(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
    let value = require(args, 0, "value", "sign")?.as_number()?;
    Ok(Value::Number(if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }))
}
```

Use `require` for positional-or-named arguments and `as_*` for type checks. Never
`unwrap` in library code.

### 3. Add tests

```rust
#[test]
fn builtin_sign() {
    assert_eq!(run_source("sign(-3)").unwrap(), Value::Number(-1.0));
    assert_eq!(run_source("sign(0)").unwrap(), Value::Number(0.0));
    assert_eq!(run_source("sign(3)").unwrap(), Value::Number(1.0));
}
```

### 4. Update the docs

Add the builtin to `docs/API_REFERENCE.md` and, if it is part of a teaching flow, to
`docs/TUTORIAL.md`. Then run `make docs`.

---

## Add a bridge convenience function

The generic `r::call("stats::median", x)` already works. Add a convenience arm only when
the call is frequent enough to deserve a shorter name.

In `Runtime::call_r`:

```rust
"median" => {
    let values = require(args, 0, "values", "r::median")?.as_array()?;
    bridge::r_call("stats::median", &[Value::Array(values.to_vec())])
}
```

In `Runtime::call_py`:

```rust
"mean" => {
    let values = require(args, 0, "values", "py::mean")?.as_array()?;
    bridge::py_call("statistics", "mean", &[Value::Array(values.to_vec())])
}
```

Bridge rules:

- Keep the process boundary. Do not embed `libR` or CPython.
- Return `Value`, not a raw Rust type.
- Convert bridge errors into `Error::Message` with the interpreter's text.
- Add a test that skips when the interpreter is unavailable.

---

## Add a language feature

Checklist for a new token or expression:

1. **Token** — add a variant to `TokenKind`, a symbol in `symbol()`, and a lexer branch.
2. **AST** — add a variant to `Expr` or `Stmt`.
3. **Parser** — add a rule at the correct precedence and include it in `parse_primary`
   or the relevant precedence method.
4. **Evaluator** — add a match arm in `Runtime::eval`.
5. **Tests** — one parser test and one runtime test at minimum.
6. **Docs** — update `docs/LANGUAGE.md` (EBNF, precedence, semantics) and
   `docs/API_REFERENCE.md` if the feature is user-facing.

Precedence changes are breaking changes. Add a test that pins the new behaviour.

---

## Extend the installer planner

The planner is a pure function:

```rust
pub fn probe() -> Profile
pub fn build_plan(profile: &Profile) -> InstallPlan
```

To add a component:

```rust
components.push(component(
    "mkl",
    "cpu.blas",
    "2026.0",
    180.0,
    520.0,
    "Intel Simplified Software License",
    "peak GEMM for Intel CPUs (optional)",
    false,
));
```

Guard it with a profile condition. Add a test that constructs a synthetic `Profile` and
asserts the component appears only when expected:

```rust
#[test]
fn nvidia_profile_plans_cuda() {
    let mut profile = probe();
    profile.gpus = vec![GpuInfo {
        vendor: "nvidia".to_string(),
        model: "test".to_string(),
        arch: Some("sm_89".to_string()),
        vram_gib: Some(24.0),
        driver: Some("550".to_string()),
        compute_supported: true,
    }];
    let plan = build_plan(&profile);
    assert_eq!(plan.profile_kind, "cuda");
    assert!(plan.components.iter().any(|component| component.id == "cublas"));
}
```

Never install a driver automatically. Never plan a backend for an unsupported
architecture.

---

## Testing strategy

| Layer | What to test |
|---|---|
| `connexon-core` | Value conversions, CSV inference, table operations, statistics |
| `connexon-syntax` | Tokenization, precedence, every new grammar rule, error positions |
| `connexon-runtime` | Evaluation, scoping, builtins, bridges, planner |
| `connexon-cli` | Exit codes, stdout/stderr, examples, plan file |

Guidelines:

- A bug fix gets a regression test.
- A new builtin gets at least one success and one type-error test.
- A bridge test must skip cleanly when the interpreter is absent.
- Prefer exact equality for integers and `1e-12` tolerance for floating point.

---

## Continuous integration

`.github/workflows/ci.yml` runs on every push and pull request:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The bridge tests skip on runners without R or Python; the CLI bridge example also
returns early with a clear message.

---

## Release checklist

1. Bump `version` in `Cargo.toml`.
2. Update `README.md`, `docs/ROADMAP.md` and `CITATION.cff` if the release changes them.
3. Run:
   ```bash
   make fmt
   make clippy
   make test
   make docs
   ```
4. Commit with a descriptive message.
5. Tag:
   ```bash
   git tag -a v0.1.0 -m "Connexon 0.1.0"
   git push origin main --tags
   ```
6. Create a GitHub release from the tag.
7. Verify a clean install:
   ```bash
   cargo install --path crates/connexon-cli --force
   cnx --version
   ```

---

## Coding conventions

- Rust 2021 edition; default `rustfmt`.
- Library code returns `Result`; no `unwrap` or `expect` outside tests.
- One error type per layer (`CoreError`, `SyntaxError`, `Error`).
- Keep R and Python behind subprocesses.
- Keep one level of parallelism; do not nest Rayon pools.
- Document every user-facing builtin in `docs/API_REFERENCE.md`.
- Keep output filenames and docs in sync with the script or command that generates them.

---

## Where to contribute

High-value areas, in order:

1. **Arrow IPC** for table transfer and Parquet support.
2. **Typed wrappers** for `DESeq2::DESeq` and common scikit-learn estimators.
3. **Lockfile** (`connexon.lock`) and environment capture.
4. **Device capability matrix** and DLPack interop.
5. **AI broker** with a local OpenAI-compatible gateway.
6. **LSP and formatter** (`cnx fmt`, `connexon-lsp`).
7. **Platform CI** for macOS and Windows.

See `docs/ROADMAP.md` for the phase boundaries.
