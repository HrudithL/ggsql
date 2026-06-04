# Agents working on TABULATE

Read [/spec/GTSQL_AGENTBUILD_SPEC.md](../gtsql/GTSQL_AGENTBUILD_SPEC.md) first.
It defines the goal, oracle, test loop, sandbox, and phase order.

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
