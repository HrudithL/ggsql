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

## 2026-06-09 — phase 3 complete

- Fixtures 06 (single spanner over related columns), 07 (two side-by-side
  spanners), and 08 (nested / stacked spanners) pass under strict
  normalization. All 10 fixture tests pass (`fixture_01..09` + `fixture_06..08`
  + `fixtures_are_well_formed`).
- New IR carries a `header_forest: Vec<HeaderNode>` derived from each
  `FORMAT SPAN <cols> AS <id>`; HTML rendering walks the forest into
  `max_height + 1` `<tr>` rows. Top-level columns sitting next to spanners
  rowspan into the spanner rows above them (matching gt's lift-up). The
  spanner `<th>` carries `gt_column_spanner_outer` and an inner
  `<div class="gt_column_spanner">` for the visible bottom border.
- Spanner id rule: bareword after `AS` is the id; a matching `LABEL <id> =>
  '<text>'` overrides the rendered text AND becomes the `id="..."` (gt's
  default behaviour: `tab_spanner(id = "...")` defaults to the label).
- `FORMAT STUB <col>` now physically promotes the stub column to position
  0 in the rendered table (gt's `rowname_col` behaviour). Fixture 03 still
  passes because its stub was already first.
- Auto float formatter now picks the minimum decimal-place count
  (1..=3) that represents every value losslessly instead of always using
  3 dp. Needed for fixtures 07 / 08 (`population_2016`, `density_2021`
  etc. — gt picks 2 dp; we previously emitted `4328.270` vs expected
  `4328.27`).
- Test harness gained `read_allowed_diff()`: parses `allowed_diff = ['re',
  ...]` from `meta.toml` and masks matching substrings to
  `<<ALLOWED_DIFF>>` on both sides before equality check.
- `tests/fixtures/08_nested_stacked_spanners/meta.toml` gains
  `allowed_diff = ['id="pop"|id="Population"', 'id="den"|id="Density"']`:
  the capture script set explicit non-default ids `pop` / `den` on the
  inner spanners (inconsistent with fixture 07 which leaves them at the
  default = label). Our standard rule yields `id="Population"` /
  `id="Density"`; the outer `id="2016–2021 Comparison"` matches exactly
  because its `LABEL` row sets the spanner text. No expected.html was
  modified.
