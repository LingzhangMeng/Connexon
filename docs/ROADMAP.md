# Connexon roadmap

## Phase 0 — vertical slice (complete)

Delivered:

- Four-crate Rust workspace: `connexon-core`, `connexon-syntax`, `connexon-runtime`, `connexon-cli`.
- Lexer, AST and parser for `.cnx`.
- Evaluator with `let`, `fn`, pipes, records, arrays, `if`, operators.
- Builtins for CSV tables, filtering, counting, statistics, JSON.
- Process-isolated R and Python bridges (`r::call`, `py::call`).
- Hardware probe for CPU, memory, GPU and toolchains.
- Consent-gated install planner with `connexon.install-plan.json`.
- 28 tests, four example programs, documentation.

Exit criteria met: `cargo build`, `cargo test`, all examples run, `cnx doctor` and `cnx install --plan-only` work.

## Phase 1 — data and packaging (6–9 months)

- Arrow-backed tables; zero-copy CSV/Parquet; schema and nullability.
- Arrow IPC between the runtime and R/Python; batched bridge calls.
- Typed stubs for the most-used R and Python functions.
- `connexon.lock` with R, Python, crate and system versions.
- Package manager integration (`renv`-style R, `uv`/`pixi` Python, `Cargo.lock`).
- LSP and Jupyter kernel.
- Conformance harness against pure R and pure Python.

Exit criteria: a real bulk RNA-seq pipeline runs with DESeq2 and scikit-learn through the typed stubs, with one lockfile.

## Phase 2 — scale and devices (9–12 months)

- Global DAG scheduler with one parallelism decision per pipeline.
- Native CPU kernels with runtime dispatch (x86-64 v2/v3/v4, NEON/SVE).
- CUDA backend (`cudarc`) and ROCm backend (HIP) behind one device trait.
- DLPack interop with PyTorch/CuPy/JAX; CUDA Graphs; multi-GPU NCCL/RCCL.
- Capability matrix with fp64/fp16/bf16 and deterministic-mode flags.
- AI broker: provider registry, policy router, provenance, local gateway.
- Local model manager: model store, backend selection, VRAM planning.

Exit criteria: the same pipeline runs on CPU and GPU with one flag; local and remote AI models are interchangeable behind the broker.

## Phase 3 — language and ecosystem (12–18 months)

- Static types and inference; modules and imports.
- Pattern matching and destructuring.
- Async and cancellation for long-running pipelines.
- Package registry and signing.
- Platform matrix: macOS notarization, Windows MSI, Linux packages, conda-forge, Spack.
- Community governance and a contribution guide.

Exit criteria: third parties publish non-trivial packages and programs.

## Risks

| Risk | Mitigation |
|---|---|
| Bridge latency dominates small calls | Batch + Arrow IPC; keep large objects across calls |
| R/Python ABI drift | Pin versions in the lockfile; process isolation |
| GPU fragmentation (`gfx803`, `gfx90a`, `sm_89`) | Capability matrix; refuse unsupported backends |
| Scope explosion | One vertical slice per phase; explicit non-goals |
| Ecosystem adoption | Reuse CRAN/PyPI; no migration required |
| Numerical divergence | Per-dtype tolerances; deterministic mode; conformance tests |

## Immediate next tasks

1. Replace JSON table transfer with Arrow IPC.
2. Add `Parquet` read/write and schema inference.
3. Add typed wrappers for `DESeq2::DESeq` and `sklearn` estimators.
4. Add a `connexon.lock` writer and `cnx env lock`.
5. Add a `cnx fmt` formatter and an LSP skeleton.
