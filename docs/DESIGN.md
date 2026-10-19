# Connexon design

## Goal

Make R packages and Python tools compose into **one typed, parallel, reproducible program**, without asking either ecosystem to give up its strengths.

## Non-goals

- Reimplementing CRAN, Bioconductor, PyPI or PyTorch.
- A general-purpose replacement for Rust, Python or R.
- Bit-identical numerical results across every device.

## Why Rust

- A small, fast native core with predictable memory behaviour.
- Mature parallelism (`rayon`), Arrow (`arrow-rs`) and GPU interop crates.
- One static binary per platform, no interpreter bootstrap.
- A credible path to a typed front end (`rowan` + `salsa` when the language grows).

## Architecture

```text
source (.cnx)
   │
   ▼
lexer → parser → AST
   │
   ▼
evaluator (scoped env, user functions)
   │
   ├── native builtins (tables, statistics, JSON, CSV)
   ├── r::  → Rscript subprocess  → JSON
   └── py:: → python3 subprocess  → JSON
```

### Decision 1 — R and Python are processes

Embedding `libR` or CPython creates three problems at once: R is not thread-safe and uses `longjmp`; Python owns the GIL; and R's GPL interacts with the runtime's licence. A subprocess boundary removes all three and is the only portable choice on Windows and macOS, where `fork` is unavailable or unsafe.

The cost is call latency (process start plus JSON). Phase 1 amortises it with **Arrow IPC and batched calls**; the process boundary stays.

### Decision 2 — JSON now, Arrow later

JSON is debuggable and dependency-free. It is the right Phase 0 contract. Phase 1 replaces it with Arrow IPC for tables and DLPack for GPU tensors, keeping the same process boundary.

### Decision 3 — one level of parallelism

Rayon is used only for large native operations (currently statistics and large table filters). BLAS is pinned to one thread per worker by policy. The runtime never nests worker pools, and bridge calls are sequential. The scheduler that will choose the level globally is a Phase 2 deliverable.

### Decision 4 — the installer is a planner

`cnx install` probes CPU, memory, GPU and toolchains, builds a profile, and emits a plan with components, versions, sizes, licences and reasons. It writes `connexon.install-plan.json` and performs no downloads in Phase 0.

Rules:

- Never install a kernel or display driver automatically.
- Never propose ROCm for an unsupported architecture (this machine's `gfx803` is the canonical example).
- Prefer user-space installation.
- Separate system-level commands and require explicit approval.

### Decision 5 — one product version, many platform artifacts

The product version is identical on every platform. Platform differences live in build metadata `(OS, arch, libc, CPU level, features)` and in per-platform dependency resolution. The installer selects exactly one artifact and refuses with a precise reason when none matches.

### Decision 6 — AI as a broker, not a vendor SDK

Planned for Phase 2: a provider-agnostic broker with a model capability registry, policy router (privacy, cost, latency), provenance record, and a local OpenAI-compatible gateway so R (`ellmer`) and Python (`openai`) share one configuration. Local models reuse `mistral.rs`, `llama.cpp`, `vLLM` or `Ollama`; the runtime owns the model store and lifecycle, not the inference engine.

## Data model

Phase 0:

```rust
enum Value {
    Null, Bool(bool), Number(f64), String(String),
    Array(Vec<Value>), Record(BTreeMap<String, Value>), Table(Table),
}
```

Phase 1:

- Arrow `RecordBatch` as the canonical table representation.
- A schema layer (`DataFrame<Schema>`) with column types and nullability.
- Zero-copy conversion at the R and Python boundaries.
- DLPack for GPU tensors; Arrow for host tables.

## Error model

Every layer has one error type: `CoreError` for values and tables, `SyntaxError` for the parser, and `Error` for the runtime, which wraps both and adds messages and I/O. The CLI prints `cnx: <message>` and exits non-zero.

## Testing

- Unit tests per crate (values, CSV/JSON, statistics, lexer, parser).
- Runtime tests for evaluation, functions, pipes, tables and both bridges.
- CLI integration tests against the built binary.
- Bridge tests skip cleanly when an interpreter is absent.
- Phase 1 adds conformance tests against pure R and pure Python results, with per-dtype tolerances.

## Security and licensing

- No automatic driver installation.
- Signed catalogue and checksum verification when downloads are implemented.
- Explicit licence display for proprietary components (CUDA, cuDNN, TensorRT).
- Process isolation limits the blast radius of a crashing interpreter.
- The core is MIT; R and Python dependencies keep their own licences at the process boundary.

## Platform strategy

- Tier 1: Linux x86_64/aarch64, macOS arm64, Windows x86_64.
- Tier 2: macOS x86_64, Windows arm64, Linux musl.
- Tier 3: compile-only for other targets.
- Baseline binaries with runtime CPU dispatch; never `-C target-cpu=native` in release builds.

## Naming

Connexon (`cnx`, `.cnx`, 连接子) — a connexon is the half-channel that two cells join to form one gap junction.
