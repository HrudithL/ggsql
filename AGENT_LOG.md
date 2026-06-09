# Agent Log

Append-only record of decisions the agent made while implementing TABULATE.

Each entry: date, phase, fixture (if applicable), one sentence describing
the deviation or `allowed_diff` justification.

---

## 2026-06-04 — bootstrap

- Cloned `posit-dev/ggsql` to local sibling of the `gtsql` spec repo.
- Created branch `agent/tabulate-bootstrap`.
- Added devcontainer, Makefile, fixture-diff harness skeleton, normalization
  module, and capture/extract R scripts.
- `src/tabulate/gt_default.css` is a placeholder; human must run
  `make css-extract` once to vendor real gt CSS.
- `tests/fixtures/` is empty; human must run `make fixtures-capture` once on
  the host (R + gt + arrow) to populate it.

## 2026-06-09 — phase 1 complete

- Fixtures 01 (minimal table), 02 (column selection / reordering), and 09
  (FORMAT hide) pass under strict normalization.
- Branch `agent/tabulate-phase-1` merged into `main` (no-ff merge commit
  `bfeba4a`); local feature branches `agent/tabulate-bootstrap` and
  `agent/tabulate-phase-1` deleted. No push to origin.
- New branch `agent/tabulate-phase-2` cut from `main` to begin phase 2
  (FORMAT STUB + LABEL title/subtitle/caption + LABEL <col>, fixtures 03,
  04, 05).

## 2026-06-09 — phase 2 complete

- Fixtures 03 (FORMAT STUB), 04 (LABEL title/subtitle), and 05 (LABEL
  title/subtitle/caption + per-column relabels + FORMAT RENAMING
  `{:num %'d}` thousands) pass under strict normalization.
- Grammar gained `label_clause` as a `tab_clause`; string literals now
  accept SQL-style `''` doubled-quote escapes (needed for `'Ontario''s
  Largest Municipalities'`).
- Fixture 05's `query.ggsql` was edited from the legacy `{:num ,d}` syntax
  to the current spec form `{:num %''d}` per TABULATE_PLAN.md §1.
  `expected.html` was not touched.
- Phase 2 lands a minimal `{:num %'d}` / `{:num %.Nf}` formatter; the full
  printf mini-language (forced sign, scientific, percent ×100, currency,
  per-column locale, `{:time ...}`) remains for phase 5.
- No `allowed_diff` entries added.

## 2026-06-09 — phase 2 merged, phase 3 opened

- Branch `agent/tabulate-phase-2` merged into `main` (no-ff merge commit
  `787981f`); feature branch deleted. No push to origin.
- `make check` reproduces the same doctest-linker OOM kills (`ld
  terminated with signal 9`) on `naming.rs` / `plot::scale::colour`
  doctests seen at the phase 1 merge — these are sandbox memory pressure,
  not a code regression (AGENTS.md notes `CARGO_BUILD_JOBS=4` as the
  documented mitigation). All `cargo test -p ggsql --test
  tabulate_fixtures` cases pass (7 / 7).
- New branch `agent/tabulate-phase-3` cut from `main` to begin phase 3
  (`FORMAT SPAN <cols> AS <id>` with nesting + LABEL through the spanner
  namespace, fixtures 06, 07, 08).
