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

## 2026-06-09 — phase 3 merged, phase 4 opened

- Branch `agent/tabulate-phase-3` merged into `main` (no-ff merge commit
  `62673f3`), followed by a fast-forward of `4f8b4c9` (example-source
  polish: bare `TABULATE` in 04, `\'` escapes in 09/10/14). Feature
  branches `agent/tabulate-phase-2` and `agent/tabulate-phase-3` deleted
  locally. `main` pushed to `origin` (`HrudithL/ggsql`).
- All 10 fixture tests pass (`fixture_01..09` + `fixture_06..08` + the
  well-formedness check). `examples/tabulate/` now ships 14 demos with
  rendered HTML at `examples/tabulate/out/index.html`.
- New branch `agent/tabulate-phase-4` cut from `main` to begin phase 4
  (`FORMAT <col> SETTING width => '<css>'` and `SETTING align => 'right'`,
  fixture 10).

## 2026-06-09 — phase 4 complete

- Fixture 10 (`FORMAT <col> SETTING width => '<css>', align => '<dir>'`)
  passes under strict normalization. All 11 fixture tests pass.
- `ColMeta` gained a `width: Option<String>` field; the HTML renderer emits
  a `<colgroup>` and swaps the table style from `width: auto;` to
  `table-layout: fixed; width: 0px;` (plus a `width="0"` attribute) when
  any column carries a width.
- `align => 'left' | 'right' | 'center'` overrides the auto-derived
  alignment in `build_table_ir`; fixture 10's alignments coincide with the
  data-type defaults so the override does not visibly differ for this
  fixture, but the override is honoured for future use.
- New example `examples/tabulate/14_widths_align.ggsql` demonstrates width
  + align settings; README updated.
- No `allowed_diff` entries added.

## 2026-06-09 — phase 5 complete

- Fixtures 11 (`{:num .3f}`), 12 (`{:num ,d}`), 13 (currency prefix
  `${:num ,d}`), 14 (percent suffix `{:num .1f}%` with ×100 scaling),
  15 (scientific `{:num .2e}` with HTML `<sup>` + Unicode minus),
  16 / 17 (`{:time ...}` on date, time, datetime), and 21 (per-column
  `SETTING locale => 'fr'`) all pass under strict normalization. 19/19
  TABULATE fixture tests green.
- New module `src/tabulate/format.rs` houses the printf/strftime
  mini-language with `CellFmt` (Numeric / Time) and `build_format()`.
  Numeric supports both legacy and `%`-prefixed printf syntax; flags
  `'`/`,` are thousands, `+` is forced sign; conversions `d`, `f`, `e`.
  Scientific output is raw HTML (`1.00&nbsp;×&nbsp;10<sup>−6</sup>`,
  U+2212 minus) and sets the column's new `raw_html` flag so the renderer
  skips HTML-escaping. Time uses chrono and parses Arrow's canonical
  ISO forms; supports `%-d` / `%-I` pad-stripping. Locale arrays for
  `en` and `fr` are hardcoded (chrono `unstable-locales` is not pulled in).
- `execute.rs`: `ColMeta` gained `raw_html: bool`; time formatters
  promote the column alignment to `right` (matching gt's auto behaviour
  for temporal data) and dispatch via Arrow `array_value_to_string` for
  `Date32`/`Date64`/`Time*`/`Timestamp*` columns so the same `{:time ...}`
  template works whether the column comes from a parquet string column
  (fixtures) or a native DuckDB temporal cast (examples).
- `html.rs`: stub column heading is always rendered `gt_left` /
  `align="left"` regardless of the data alignment of the stub body cells
  (matches gt's convention seen in fixture 14, where the stub is numeric
  but its column heading is still left-aligned). Body cells use a column's
  `raw_html` to choose between `html_escape(value)` and the raw string.
- `tests/fixtures/14_percent_formatting_from_proportions/query.ggsql`
  rewritten to operate on the materialized `monthly` table (the captured
  query referenced `pizzaplace.date` which is not in `data.parquet`).
  Expected HTML untouched.
- New examples 17–24 demonstrate each phase-5 surface
  (`17_num_decimals`, `18_num_thousands`, `19_currency`, `20_percent`,
  `21_scientific`, `22_dates`, `23_datetime`, `24_french_locale`); README
  updated; `examples/tabulate/out/index.html` regenerated.
- No `allowed_diff` entries added.

## 2026-06-10 — phase 6 complete

- Fixtures 18 (`RENAMING null => '<text>'`), 19 (`RENAMING 0 => '<text>'`
  composed with `* => '{:num ,d}'`), and 20 (`RENAMING '<value>' =>
  '<text>'` exact match) pass under strict normalization. 22/22 TABULATE
  fixture tests green.
- `build_table_ir` now builds four substitution maps in addition to
  `format_overrides`: `null_subst`, `zero_subst`, `numeric_substs`, and
  `literal_substs`. Row rendering consults them in spec precedence order
  (literal > null > 0 > `*`) before falling through to the cell formatter.
- Substitution RHS strings pass through a new `smart_text()` helper that
  collapses `---` → em-dash, `--` → en-dash, `...` → ellipsis. This
  matches gt's `sub_missing()` text processing — needed for fixture 18
  where `RENAMING null => '---'` renders as `—`.
- Grammar fix in `tree-sitter-ggsql/grammar.js`: the `string` rule is now
  wrapped in `token(...)` so tree-sitter extras (whitespace + SQL `--`
  comments) are not inserted inside string literals. Without this, a
  literal like `'---'` parsed as `'-` + `--<comment>` + missing closing
  `'`. All 103 tree-sitter corpus tests still pass.
- New examples `25_replace_missing`, `26_replace_zero`,
  `27_direct_value_mapping`; README updated;
  `examples/tabulate/out/index.html` regenerated.
- No `allowed_diff` entries added.
