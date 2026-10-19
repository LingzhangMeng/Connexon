# Handoff — Project Handoff Document

**Project:** Connexon 0.1.0 (Phase 0)

**Updated:** 2026-10-19

**Repository:** https://github.com/LingzhangMeng/Connexon

## 1. Objective (What we are doing)

Build **Connexon**, a Rust runtime and language that composes R packages and Python tools into one typed, parallel, reproducible program for bioinformatics, data science and AI. R keeps its statistical ecosystem; Python keeps its model ecosystem; Connexon owns the data flow, process isolation, parallelism and reproducibility between them. Phase 0 is a runnable vertical slice (parser, evaluator, bridges, hardware probe, install planner, CLI), now published as an MIT-licensed public repository with a full documentation set and brand. Success means the repository is public, `cargo test --workspace` passes, the examples run, and `cnx doctor` / `cnx install --plan-only` work.

## 2. Completed Items (What has been done)

### Core implementation

- **Workspace** — four crates: `connexon-core`, `connexon-syntax`, `connexon-runtime`, `connexon-cli`; shared dependency versions; release profile with thin LTO.
- **Core** — `Value` and `Table`, CSV type inference and round trip, JSON conversion (arrays of records become tables), Rayon-backed statistics, typed errors.
- **Syntax** — lexer with line/column tracking, AST, precedence-climbing parser supporting `let`, `fn`, `|>`, `::`, records, arrays, `if`, named arguments.
- **Evaluator** — scoped locals, user functions, short-circuit logic, type-checked arithmetic, pipes that inject the left value as the first argument, table and statistics builtins.
- **Bridges** — `r::call` and `py::call` spawn `Rscript` / `python3` as separate processes, exchange JSON through temporary files, support R package namespaces (`stats::sd`) and Python modules, and return typed values or clear errors.
- **Probe and planner** — CPU topology/ISA/NUMA, memory, `lspci`/NVIDIA/ROCm/AMD VRAM, toolchains; a pure planner that emits components, sizes, licences, notes and manual system commands. It correctly refuses ROCm for `gfx803`.
- **CLI** — `cnx run`, `cnx eval`, `cnx doctor`, `cnx install [--plan-only|--yes]`; writes `connexon.install-plan.json`.
- **Examples** — `hello.cnx`, `functions.cnx`, `table.cnx`, `bridge.cnx`, `pipeline.cnx`, plus `examples/data/de_sample.csv`.

### Verification

- **Tests:** 28 passing — 4 core, 10 runtime (including live R and Python bridges), 7 syntax, 7 CLI integration.
- **Quality gates:** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Verified outputs:
  - `examples/hello.cnx` → 15, 3, 1.5811388300841898
  - `examples/functions.cnx` → 25, `large`
  - `examples/table.cnx` → 4 significant rows, table previews
  - `examples/bridge.cnx` → R mean 3, R sd 1.58113883008419, Python stdev 1.5811388300841898
  - `examples/pipeline.cnx` → 4, 0.775, 1.840335114410779
  - `cnx doctor` → Intel i5-10400, 6C/12T, 61.6 GiB, AMD WX 3200 `gfx803` compute=false, cpu-only plan
  - `cnx install --plan-only` → `connexon.install-plan.json`

### Documentation and metadata

- **README.md** — 475 lines: motivation, status, prerequisites, quick start, language tour, CLI reference, architecture, probe/installer, repository layout, documentation index, examples, testing, troubleshooting, roadmap, contributing, citation, developer, licence.
- **docs/TUTORIAL.md** — 13 lessons: values, bindings, functions/recursion, arrays/records, pipes, `if`, tables, filtering/writing, R, Python, JSON, end-to-end pipeline, errors, exercises.
- **docs/API_REFERENCE.md** — every builtin and bridge signature, conversion rules, operators, error messages, Phase 0 limits.
- **docs/DEVELOPER_GUIDE.md** — architecture, adding builtins/parser features/planner components, testing, release checklist.
- **docs/FAQ.md** — design rationale, DESeq2/PyTorch questions, GPU troubleshooting, project questions.
- **docs/LANGUAGE.md** — formal EBNF, precedence table, type rules, scoping.
- **docs/DESIGN.md** — architecture decisions: process isolation, JSON→Arrow, one parallelism level, installer as planner, platform artifacts, AI broker.
- **docs/ROADMAP.md** — Phase 1–3 plan and risks.
- **docs/BRAND.md** — mark concept, palette, clear space, usage rules.
- **11 `.docx` twins** generated from Markdown with Times New Roman + 仿宋 + red Consolas code; `make docs` is warning-free using the vendored template `docs/templates/reference_tnr_fangsong_redcode.docx`.
- **LICENSE** — MIT, `Copyright (c) 2026 Prof. Dr. rer. nat. Lingzhang Meng`.
- **CITATION.cff**, **AUTHORS.md**, **handoff.md**.
- **CI** — `.github/workflows/ci.yml` runs fmt, clippy and tests on Ubuntu.

### Brand

- **Mark** — hexagonal connexon split into teal (R) and amber (Python) halves, joined by an ivory channel with a navy pore on a navy field (Rust).
- **Assets** — `assets/logo.svg`, `logo.png` (1024), `logo-512.png`, `logo-128.png`, `logo-transparent.svg/.png`, `favicon.svg`, `social-preview.png` (1280×640).
- README logo is centered with a `<div align="center">` wrapper around a Markdown image, so GitHub centers it and `README.docx` still embeds it.

### Publication and outreach

- **GitHub** — public repository at https://github.com/LingzhangMeng/Connexon, default branch `main`, MIT licence.
- **Commits** — `34200e0` (package + docs + CI), `9e1be15` (mark, brand guide, social preview), `ffdacfe` (centered README mark).
- **Deploy key** — dedicated `~/.ssh/connexon_deploy` (private key mode 600), SSH alias `github-connexon` using `ssh.github.com:443`; `git push` works.
- **WeChat Moments draft** — `/home/meng/Desktop/Connexon_微信朋友圈文稿.md` and `.docx`: industry pain points, R/Python advantages, use cases (vector/graph, NLP, medical big data, medical AI, CPU multicore, CUDA/ROCm), download link, inventor; plus short/technical versions, nine-grid plan, Q&A and publication tips.

### Decisions made and rationale

- **Process isolation for R/Python** — avoids GIL, R's single-threaded/longjmp model, crash propagation, GPL linking; the only portable design on Windows/macOS.
- **JSON now, Arrow/DLPack later** — debuggable Phase 0 contract; Phase 1–2 keeps the same process boundary with zero-copy formats.
- **One level of parallelism** — Rayon for large native operations; BLAS pinned to 1 thread per worker; never nest pools.
- **Installer is a planner** — never install drivers automatically; never plan a backend the hardware cannot run.
- **MIT licence** — per user request; matches the published repository.
- **Name Connexon** — `cnx`, `.cnx`, 连接子; the gap-junction metaphor.

## 3. Current Blockers (Where we are stuck)

**None blocking.** Deliberate Phase 0 limits and pending manual actions:

- **Tables are row-oriented in memory**; Arrow is Phase 1. Impact: memory use on large tables.
- **Bridge payloads are JSON**; complex objects (DESeqDataSet, DataFrame, tensors) need Phase 1 adapters. Impact: cannot pass fitted models or assay objects.
- **The installer plans but does not download**. Impact: `cnx install` is a dry run.
- **GPU execution and AI calls are not implemented**; only detection/planning exists. Impact: CUDA/ROCm and AI training are Phase 2.
- **This machine is CPU-only for heavy compute**: AMD WX 3200 is `gfx803`, unsupported by modern ROCm. GPU work must be tested on a CUDA or supported-ROCm node.
- **GitHub social preview** — upload `assets/social-preview.png` manually under Settings → General → Social preview. Not done yet.
- **First GitHub Actions run** — status not yet observed. 【To be confirmed: check the Actions tab after the latest push.】
- **Release tag `v0.1.0`** — not created yet.
- **`ssh -T git@github-connexon`** returns `Connection reset by peer` on port 443, but `git ls-remote` and `git push` authenticate and work. Do not treat the `ssh -T` banner as a blocker.

## 4. Next Steps (What to do next)

- **P0 — Create the GitHub release.** Tag `v0.1.0` at `ffdacfe`, push the tag, and publish release notes from the README. Owner: TBD. Trigger: after confirming CI.
- **P0 — Upload the social preview** (`assets/social-preview.png`) and add repository topics/description (`bioinformatics`, `rust`, `r`, `python`, `reproducible-research`). Owner: TBD.
- **P0 — Arrow IPC + Parquet.** Replace JSON table transfer in `crates/connexon-runtime/src/bridge.rs`; add Parquet read/write to `connexon-core`. Resume at `docs/ROADMAP.md` Phase 1.
- **P1 — `connexon.lock`.** Add `cnx env lock` recording R, Python, crate and system versions; resume in `crates/connexon-cli/src/main.rs` and `connexon-runtime/src/system.rs`.
- **P1 — Typed wrappers.** Add `DESeq2::DESeq` and a scikit-learn estimator path; prove on real bulk RNA-seq data. Resume at `crates/connexon-runtime/src/lib.rs` (`call_r` / `call_py`).
- **P1 — `cnx fmt` and LSP skeleton.** Resume in `crates/connexon-cli` and a new `connexon-lsp` crate.
- **P2 — Device capability matrix + DLPack.** CUDA via `cudarc`, ROCm via HIP; same `.cnx` on CPU/GPU. Resume at `connexon-runtime/src/system.rs` (profile) and `docs/DESIGN.md`.
- **P2 — AI broker.** OpenAI-compatible transport, capability registry, policy router, provenance, local gateway. Resume at `docs/ROADMAP.md` Phase 2.
- **P3 — Static types, modules, registry, platform packages.** Resume at `docs/ROADMAP.md` Phase 3.

## 5. Pitfalls & Anti-patterns (Never repeat these)

- **Never write `request$function` in R.** `function` is reserved; use `request[["function"]]`. This broke the first R bridge run.
- **Never write `String + String` in Rust.** Use `a + &b`; the compiler rejects the owned concatenation.
- **Never embed R or Python.** Process isolation avoids R's thread model, Python's GIL, `longjmp` across Rust frames and GPL linking.
- **Never nest parallelism.** One Rayon level; BLAS pinned to 1 thread per worker.
- **Never install a driver automatically.** Detect, explain, and print the command.
- **Never propose ROCm for `gfx803`.** Probe the architecture, not the vendor.
- **Never trust `nproc` alone.** Read physical cores and cgroup quotas.
- **Never let a bridge failure panic the runtime.** Return a typed error with the interpreter's message.
- **Never use `-C target-cpu=native` in release builds.** Use baseline binaries plus runtime dispatch.
- **Never treat `ssh -T` as the auth test for this deploy key.** It resets on port 443 here; use `git ls-remote origin` instead.
- **Never put a raw HTML `<img>` in the README if the `.docx` must embed it.** Pandoc drops it; use `![alt](assets/logo-128.png)`.
- **Never use `../assets/...` in `docs/BRAND.md` without a resource path.** Run pandoc with `--resource-path=docs`.
- **Never use SVG shields.io badges in a docx pipeline without `rsvg-convert`.** Pandoc warns and drops them; use plain text or PNG.
- **Never commit every regenerated `.docx` blindly.** `make docs` changes timestamps; restore unchanged binaries to keep commits clean.
- **Never name a probe script `docx.py` (or `numpy.py`).** It shadows the real package via `sys.path[0]`; `find_spec` gives a false positive.
- **Never overclaim CUDA/ROCm execution or AI training.** Phase 0 is detection/planning only.
- **Never put the GitHub link only in the WeChat body.** Put it in the first comment and pin it; links are folded.
- **Never commit `target/` or `connexon.install-plan.json`.** Both are git-ignored.

## 6. Key Resources & Environment Info

- **Project root:** `/run/media/meng/Bioinformatics/New_language/connexon`
- **Git remote:** `git@github-connexon:LingzhangMeng/Connexon.git`
- **Branch:** `main`; latest commit `ffdacfe` (centered README mark), preceding `9e1be15` (brand), `34200e0` (package).
- **Deploy key:** `~/.ssh/connexon_deploy` (private, mode 600); public key `~/.ssh/connexon_deploy.pub`; SSH alias `github-connexon` in `~/.ssh/config` (HostName `ssh.github.com`, Port 443, IdentitiesOnly yes).
- **Toolchain:** Rust 1.95.0 (edition 2021); crates.io reachable.
- **Dependencies:** `serde`, `serde_json`, `csv`, `tempfile`, `thiserror 2`, `rayon`, `clap 4`.
- **R:** 4.6.1 with `jsonlite`.
- **Python:** 3.13.13 (miniconda), standard library.
- **Docs tooling:** pandoc 3.7.0.2; ImageMagick `magick`; `python-docx` 1.2.0 (installed from Aliyun mirror); no `rsvg-convert`.
- **Hardware:** Intel i5-10400, 6C/12T, 61.6 GiB RAM, Intel UHD 630, AMD WX 3200 (`gfx803`, 4 GiB, no ROCm toolchain) — treat as CPU-only for heavy compute.
- **Commands:** `make build`, `make test`, `make fmt`, `make clippy`, `make run`, `make doctor`, `make install-plan`, `make docs`.
- **CI:** `.github/workflows/ci.yml` (fmt + clippy + test on ubuntu-latest).
- **Docs:** `README.md`, `docs/TUTORIAL.md`, `docs/API_REFERENCE.md`, `docs/DEVELOPER_GUIDE.md`, `docs/FAQ.md`, `docs/LANGUAGE.md`, `docs/DESIGN.md`, `docs/ROADMAP.md`, `docs/BRAND.md`; all with `.docx` twins.
- **Brand assets:** `assets/logo.svg`, `logo.png`, `logo-512.png`, `logo-128.png`, `logo-transparent.svg/.png`, `favicon.svg`, `social-preview.png`.
- **Desktop outreach:** `/home/meng/Desktop/Connexon_微信朋友圈文稿.md` and `.docx`.
- **Artifacts (git-ignored):** `target/`, `connexon.install-plan.json`.
- **Cross-machine resume:** clone `https://github.com/LingzhangMeng/Connexon.git`; install Rust, R (`jsonlite`) and Python; run `cargo test --workspace`; run `make docs` only if pandoc is installed. No secrets are stored in the repository; the deploy key is local to this machine.
- **Developer:** Prof. Dr. rer. nat. Lingzhang Meng, Research Center for Medical Big Data and Artificial Intelligence, The People's Hospital of Guangxi Zhuang Autonomous Region (Guangxi Academy of Medical Sciences); 孟令章教授[广西壮族自治区人民医院（广西医学科学院）,医学大数据与人工智能研究中心].

## 7. Startup Instruction for the Next Session

> **When starting a new session, the very first message must be: "Read handoff.md first."**

Then restate the current state from this document before changing anything. Check, in this order:

```bash
cd /run/media/meng/Bioinformatics/New_language/connexon
git status
git log --oneline -3
cargo test --workspace
```

If the previous session was interrupted, check the logs and the test output first; those are the authoritative record of what actually works. If the repository state is unclear, run `git fetch origin && git log --oneline origin/main -3` before making changes.
