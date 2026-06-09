# Agents working on TABULATE

Read in this order:

1. [/spec/GTSQL_AGENTBUILD_SPEC.md](../gtsql/GTSQL_AGENTBUILD_SPEC.md) —
   goal, oracle, normalization rule, phase order, sandbox, pass gate.
2. [/spec/GTSQL_PLAN.md](../gtsql/GTSQL_PLAN.md) — language reference
   (authoritative for syntax and semantics).
3. [/spec/GTSQL_EXAMPLES.qmd](../gtsql/GTSQL_EXAMPLES.qmd) — example corpus
   (each example is a fixture).
4. [TABULATE_PLAN.md](TABULATE_PLAN.md) — implementation surfaces in this
   repo (which file does what), phase-by-phase scope, and implementation
   notes the spec leaves open.

## System prompt (paste into Copilot agent / Codex / Claude Code)

You are implementing TABULATE in posit-dev/ggsql per
/spec/GTSQL_AGENTBUILD_SPEC.md. Your ground truth is the captured HTML in
tests/fixtures/*/expected.html. Your job is to make `make check` pass.

Hard rules:
- Never modify tests/fixtures/*/expected.html.
- Never modify src/tabulate/gt_default.css.
- Never weaken src/tabulate/test_normalize.rs to make a diff pass.
- Never add an `allowed_diff` entry without writing a justification line in
  AGENT_LOG.md.
- Commit only to a feature branch named `agent/tabulate-phase-<N>`. Open a PR
  when a phase is green. Never push to main, never force-push, never rewrite
  history.
- Follow the phase order in /spec/GTSQL_AGENTBUILD_SPEC.md §5 strictly.

On each iteration: pick the lowest-numbered failing fixture, change the
minimum code needed, re-run the targeted test (`cargo test --test
tabulate_fixtures -- <fixture>`), then `make check`. If a previously-green
fixture regresses, revert and try a smaller change.

Stop conditions:
- All fixtures pass under strict normalization → open a PR for the phase.
- 50 iterations without phase advancement → stop, summarize blockers in
  AGENT_LOG.md, hand back to human.
- 6h wall clock → same.

## Where things live

- Spec & examples: `/spec/` (read-only mount of the gtsql repo).
- Plan (semantics reference): `/spec/GTSQL_PLAN.md`.
- Example corpus: `/spec/GTSQL_EXAMPLES.qmd`.
- Captured fixtures: `tests/fixtures/<NN_slug>/{query.ggsql, data.parquet, expected.html, meta.toml}`.
- New code subtree: `src/tabulate/`.
- Grammar entry to extend: `tree-sitter-ggsql/grammar.js` (rule `query` is
  the top-level seq; add `repeat($.tabulate_statement)` alongside
  `repeat($.visualise_statement)`).
- AST: extend `src/plot/` with `TabulateStmt`, `FormatClause`, etc., or add a
  parallel `src/tabulate/ast.rs` if cleaner — agent decides.
- Parser lowering: `src/parser/builder.rs`.
- HTML writer: `src/tabulate/html.rs`.

## Pass gate

`make check` runs fmt, clippy, tree-sitter tests, all cargo tests, and the
fixture-diff suite.

## Using cargo efficiently

This workspace is heavy: `duckdb` and `rusqlite` are compiled from C++/C
source via the `bundled` feature, the dependency graph is large (arrow,
parquet, geozero, adbc, palette, ...), and a full `target/` directory can
exceed 60 GB. Treat cargo invocations as expensive and minimise them.

**Default to `cargo check`, not `cargo build` or `cargo test`.** `check`
skips the linker, which is where most wall time goes (a `ggsql` test binary
links a ~1–2 GB `libduckdb_sys` rlib). Use `check` to validate edits and
only run `test` when you actually need to execute code.

**Scope every invocation:**
- One package: `cargo check -p ggsql` (not `--workspace`).
- One test binary: `cargo test -p ggsql --test tabulate_fixtures`.
- One fixture: `cargo test -p ggsql --test tabulate_fixtures -- <fixture>`
  (filter is a substring match on test names).
- Skip doctests when you don't need them: add `--lib --tests --bins`.

**Avoid feature creep.** The `ggsql` crate defaults to
`adbc,duckdb,sqlite,vegalite,parquet,builtin-data,odbc,spatial`. For TABULATE
work you typically only need `duckdb,parquet,vegalite,builtin-data`. When a
task does not touch a reader, build with
`--no-default-features --features duckdb,parquet,vegalite,builtin-data` to
cut compile time substantially. Run the full default feature set only as
part of `make check` before opening a PR.

**Do not run `cargo clean`** unless you are reclaiming disk or have reason
to believe the cache is corrupt. Incremental compilation is the only thing
making this workspace tolerable.

**Do not run cargo commands in parallel** from the agent (do not background
one `cargo` and start another). Cargo serialises on a per-target lock and
the second invocation will simply block, doubling your wall time for no
gain. Run one, wait, then run the next.

**Parallelism is set by `nproc` by default.** If you see OOM kills during
`libduckdb-sys` or `rusqlite-sys` C++ compilation, set
`CARGO_BUILD_JOBS=4` (or lower) in the environment for that command rather
than editing `.cargo/config.toml` — that config is shared with other
contributors.

**Heuristic for `make check`:** run it once at the start of a phase to
confirm a clean baseline, then iterate with targeted `cargo test` calls,
and only re-run `make check` when you believe the phase is complete or
before committing. Do not run `make check` after every edit.
