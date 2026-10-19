# Handoff — Connexon Phase 0

## 1. Objective (What we are doing)

Build **Connexon**, a Rust runtime and language that composes R packages and Python tools into one typed, parallel, reproducible program for bioinformatics, data science and AI. Phase 0 delivers a runnable vertical slice: parser, evaluator, process-isolated bridges, hardware probe, install planner and CLI. Success means `cargo build`, `cargo test`, the examples run, and `cnx doctor` / `cnx install --plan-only` work.

## 2. Completed Items (What has been done)

- **Workspace** — four crates (`connexon-core`, `connexon-syntax`, `connexon-runtime`, `connexon-cli`) with shared dependency versions and a release profile.
- **Core** — `Value` and `Table`, CSV type inference and round trip, JSON conversion (arrays of records become tables), Rayon-backed statistics, typed errors.
- **Syntax** — lexer with line/column tracking, AST, precedence-climbing parser supporting `let`, `fn`, `|>`, `::`, records, arrays, `if`, named arguments.
- **Evaluator** — scoped locals, user functions, short-circuit logic, type-checked arithmetic, pipes that inject the left value as the first argument, table and statistics builtins.
- **Bridges** — `r::call` and `py::call` spawn `Rscript` / `python3`, exchange JSON through temp files, support R package namespaces (`stats::sd`) and Python modules, and return typed values or clear errors.
- **Probe and planner** — CPU topology/ISA/NUMA, memory, `lspci`/NVIDIA/ROCm/AMD VRAM, toolchains; a pure planner that emits components, sizes, licences, notes and manual system commands. It correctly refuses ROCm for `gfx803`.
- **CLI** — `cnx run`, `cnx eval`, `cnx doctor`, `cnx install [--plan-only|--yes]`; writes `connexon.install-plan.json`.
- **Examples** — `hello.cnx`, `functions.cnx`, `table.cnx`, `bridge.cnx`, plus `examples/data/de_sample.csv`.
- **Tests** — 28 passing: 4 core, 10 runtime, 7 syntax, 7 CLI integration.
- **Docs** — `README.md`, `docs/LANGUAGE.md`, `docs/DESIGN.md`, `docs/ROADMAP.md`, with `.docx` twins.

Verified outputs:

- `examples/hello.cnx` → 15, 3, 1.5811388300841898
- `examples/functions.cnx` → 25, `large`
- `examples/table.cnx` → 4 significant rows, table previews
- `examples/bridge.cnx` → R mean 3, R sd 1.58113883008419, Python stdev 1.5811388300841898
- `cnx doctor` → Intel i5-10400, 6C/12T, 61.6 GiB, AMD WX 3200 `gfx803` compute=false, cpu-only plan
- `cnx install --plan-only` → `connexon.install-plan.json` (2.9 KB)

## 3. Current Blockers (Where we are stuck)

None blocking. Deliberate Phase 0 limits:

- Tables are row-oriented in memory; Arrow is Phase 1.
- Bridge payloads are JSON; complex objects (DESeqDataSet, DataFrame, tensors) need Phase 1 adapters.
- The installer plans but does not download.
- GPU execution and AI calls are not implemented; only GPU detection is present.
- This machine is CPU-only for heavy compute: the AMD WX 3200 is `gfx803`, unsupported by modern ROCm. GPU work must be tested on a CUDA or supported-ROCm node.

## 4. Next Steps (What to do next)

- **P0** — Replace JSON table transfer with Arrow IPC and add Parquet read/write.
- **P0** — Add a `connexon.lock` writer (`cnx env lock`) recording R, Python, crate and system versions.
- **P1** — Add typed wrappers for `DESeq2::DESeq` and a scikit-learn estimator, proving the vertical slice on real data.
- **P1** — Add `cnx fmt` and an LSP skeleton.
- **P1** — Add schema-aware tables (`DataFrame<Schema>`) with nullability.
- **P2** — Implement the device capability matrix and DLPack interop; CUDA first, ROCm second.
- **P2** — Implement the AI broker (OpenAI-compatible transport, capability registry, policy router, provenance) and the local OpenAI-compatible gateway.

## 5. Pitfalls & Anti-patterns (Never repeat these)

- **Never write `request$function` in R.** `function` is reserved; use `request[["function"]]`. This broke the first R bridge run.
- **Never embed R or Python.** Process isolation avoids R's thread model, Python's GIL, `longjmp` across Rust frames and GPL linking.
- **Never nest parallelism.** One Rayon level; BLAS pinned to 1 thread per worker.
- **Never install a driver automatically.** Detect, explain, and print the command.
- **Never propose ROCm for `gfx803`.** Probe the architecture, not the vendor.
- **Never trust `nproc` alone.** Read physical cores and cgroup quotas.
- **Never let a bridge failure panic the runtime.** Return a typed error with the interpreter's message.
- **Never use `-C target-cpu=native` in release builds.** Use baseline binaries plus runtime dispatch.

## 6. Key Resources & Environment Info

- **Project root:** `/run/media/meng/Bioinformatics/New_language/connexon`
- **Rust:** 1.95.0 (2026-03-21); edition 2021; crates.io reachable.
- **Dependencies:** `serde`, `serde_json`, `csv`, `tempfile`, `thiserror 2`, `rayon`, `clap 4`.
- **R:** 4.6.1 with `jsonlite` available.
- **Python:** 3.13.13 (miniconda) with the standard library.
- **Hardware:** Intel i5-10400, 6C/12T, 61.6 GiB RAM, Intel UHD 630, AMD WX 3200 (`gfx803`, 4 GiB, no ROCm toolchain).
- **Commands:** `make build`, `make test`, `make fmt`, `make clippy`, `make run`, `make doctor`, `make install-plan`, `make docs`.
- **Artifacts:** `connexon.install-plan.json` (generated, git-ignored), `target/` (git-ignored).
- **Developer:** Prof. Dr. rer. nat. Lingzhang Meng, Research Center for Medical Big Data and Artificial Intelligence, The People's Hospital of Guangxi Zhuang Autonomous Region (Guangxi Academy of Medical Sciences); 孟令章教授[广西壮族自治区人民医院（广西医学科学院）,医学大数据与人工智能研究中心].
- **Naming:** Connexon, CLI `cnx`, extension `.cnx`, Chinese 连接子, GitHub org `connexon-lang`, domain `connexon.dev` (likely available at check time).

## 7. Startup Instruction for the Next Session

> **When starting a new session, the very first message must be: "Read handoff.md first."**

Then restate the state above before changing anything. If the previous session was interrupted, check `cargo test --workspace` and the example outputs first; those are the authoritative record of what works.
