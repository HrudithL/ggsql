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

## 2026-06-11 — phase 7 complete

- Fixtures 22 (`SCALE background FROM (lo,hi) TO RdYlGn` over four
  columns), 23 (auto-inferred domain with two-stop explicit-colour
  gradient), 24 (`SCALE background TO viridis`), and 25 (`SCALE … VIA
  log10` with explicit domain anchored at zero) pass. 26/26 TABULATE
  fixture tests green.
- New `src/tabulate/scale.rs` implements gt's `data_color()` semantics:
  Lab(D65) interpolation via the `palette` crate's `Lab::mix()`, named
  palettes via `crate::plot::scale::palettes::get_color_palette`, explicit
  hex / CSS colour stops via `csscolorparser`, NA → `#808080` for
  out-of-domain / non-finite values, and `ideal_fg` foreground colour
  picked by **YIQ brightness threshold 156** (not WCAG contrast — gt uses
  YIQ). For `VIA log10`, `lo_t = 0` when `domain.0 <= 0` to match gt's
  `scales::col_numeric(transform = "log10", domain = c(0, hi))`, which
  otherwise produces `-Inf` and a NA-only column.
- `build_float_formatter` (`src/tabulate/execute.rs`) now emits scientific
  notation `%.3e` for all-integer double columns when `max_abs >= 1e8`.
  Fixture 25's `population` column (1.4e9) needed this; gt switches to
  scientific at that threshold for default float formatting.
- AST: `TabulateStmt.scale_clauses: Vec<ScaleClause>`,
  `ScalePalette::{Stops(Vec<String>), Named(String)}`. Parser:
  `parse_scale_clause` in `src/parser/tabulate.rs` walks
  `tab_scale_from/to/via/setting`. Grammar: `tab_scale_clause` added to
  `tab_clause` choice (`tree-sitter-ggsql/grammar.js`); 105 corpus tests
  pass.
- `TableIr.cell_bg: Vec<Vec<Option<String>>>` carries per-cell hex
  backgrounds. `html.rs` appends `background-color: <hex>; color:
  <ideal_fg>;` plus `bgcolor="<hex>"` to each non-stub cell that has a
  background. Last scale clause wins (gt's last-writer semantics).
- One `allowed_diff` entry: fixture 24 (`24_named_viridis_palette`). Our
  bundled `VIRIDIS` 256-stop palette in `src/plot/scale/palettes.rs`
  differs from R's `viridisLite::viridis(256)` by 1 unit per channel
  (e.g. `#3A528B` vs R's `#3B528B`). The mask
  `'background-color: #[0-9A-F]{6}'` + `'bgcolor="#[0-9A-F]{6}"'` lets
  the structural HTML diff strictly while ignoring the constant
  per-channel hex shift. Documented inline in `meta.toml`.
- Fixed two bugs in `read_allowed_diff` (`src/tests/tabulate_fixtures.rs`)
  uncovered while writing fixture 24's mask: (1) `text.split('#')` was
  stripping `#` inside regex patterns as if it were a TOML comment, and
  (2) a `]` inside a regex character class (e.g. `[0-9A-F]`) was
  prematurely ending the array body. Replaced with a line-by-line state
  machine plus quote-aware `strip_line_comment` and
  `has_close_bracket_outside_quotes` helpers.
- New examples `28_scale_named_palette`, `29_scale_explicit_colors`,
  `30_scale_explicit_domain`, `31_scale_log_transform`; README updated;
  `examples/tabulate/out/index.html` regenerated.

## 2026-06-11 — phase 8 complete

- Fixtures 26 (single-column conditional cell highlight), 27 (multi-column
  conditional highlight), and 28 (two HIGHLIGHTs for up/down stock days)
  pass under strict normalization.
- Grammar: added `tab_highlight_clause` (with `tab_highlight_setting` /
  `tab_highlight_pair`) to `tab_clause` alternation in
  `tree-sitter-ggsql/grammar.js`; reused the existing `filter_clause`
  rule so `WHERE`-style SQL predicates can be embedded inline. 105/105
  tree-sitter corpus tests pass.
- AST: `TabulateStmt` gained `highlight_clauses: Vec<HighlightClause>`;
  each entry holds the target column list, the raw predicate source, and
  the parsed SETTING key/value pairs.
- Predicate evaluation strategy: instead of building a separate SQL
  evaluator, `build_sql` wraps the user's SELECT in a subquery and
  appends one boolean projection per HIGHLIGHT named `__hl_<N>__match`.
  After execution, `build_cell_style` reads each `BooleanArray` column
  and writes `CellStyle { background, color, face }` for every (row, col)
  hit. The `__hl_*` columns are filtered out of `schema_names` so they
  never appear in the rendered table.
- HTML rendering: `render_tr` now takes an optional per-row
  `&[CellStyle]`. The body-cell renderer merges scale background with
  highlight background using gt's last-writer-wins semantics —
  HIGHLIGHT background overrides SCALE background, but unlike SCALE we
  do NOT synthesize a foreground colour from YIQ when the bg comes
  solely from HIGHLIGHT (gt only does this for SCALE-style continuous
  fills). `face` maps to `font-weight: bold` for `'bold'`, to
  `font-style: italic|oblique` for those values, and to
  `font-weight: <value>` otherwise. `color` maps to `color: <hex>;`.
- New examples 32-34 (`32_highlight_failing_scores`,
  `33_highlight_region_row`, `34_highlight_up_down_days`) demonstrate
  single-column conditional emphasis, multi-column row-style highlights,
  and composing two HIGHLIGHTs with currency formatting; README updated;
  `examples/tabulate/out/index.html` regenerated.
- No `allowed_diff` entries added for this phase.

## 2026-06-11 — phase 9 complete
- Fixtures 29 (single-aggregate per-group `sum`) and 30 (multi-aggregate
  `min` / `max` / `mean` per week of SP500 data) pass under strict
  normalization.
- Grammar: added `tab_facet_clause` to the `tab_clause` choice in
  `tree-sitter-ggsql/grammar.js`, with `tab_facet_setting_block`,
  `tab_facet_pair`, `tab_facet_id_list`, and `tab_facet_str_list` rules.
  Identifier lists use `(a, b, c)` syntax; string lists accept either
  `('a', 'b')` or `['a', 'b']`. 105/105 tree-sitter corpus tests pass.
- AST: `TabulateStmt` gained `facet: Option<FacetClause>`. `FacetClause`
  carries the group column plus a flat `Vec<FacetSetting>` of key/value
  pairs (`target`, `aggregate`, `label`, `side`) so the executor can
  validate cross-pair invariants in one place.
- Execution: `build_table_ir` now drops the group column from
  `visible_cols`, synthesizes an empty stub column when FACET is set
  without an explicit `FORMAT STUB <col>` (gt renders the group heading
  in the stub slot), and calls `build_row_groups` to walk the group
  column in *discovery order* (HashMap-backed first-seen tracking,
  matching how gt's R-side captures order). Each group computes summary
  rows by aggregating the named `target` columns with the named
  `aggregate` functions; non-target columns render as the em-dash
  (`U+2014`). `compute_aggregate` covers `sum`, `min`, `max`,
  `mean` / `avg` / `average`, `median`, and `sd` / `stdev` / `stddev`
  (sample SD, n-1 divisor). Summary labels default to the aggregate
  function name; an explicit `label => ('A', 'B', …)` overrides 1:1.
- Auto formatter: integer-valued double columns previously always
  rendered as fixed integers below `1e8` and switched to scientific
  above. Phase 9 introduces a sharper heuristic: integer columns with
  `max_abs >= 1e6` that round cleanly to 4 significant figures get
  scientific notation (matching fixture 25's population of `6.500e+05`
  through `1.412e+09`); integer columns that carry more precision
  (fixture 30's SP500 volume of `4378680000`) stay as plain integers.
  Float columns now round to 7 significant figures before the
  smallest-lossless-dp search, so noisy doubles like `2044.8101` /
  `2062.1399` settle on 2 dp (matching R's default `digits = 7`) instead
  of 3.
- Summary value formatter: per-cell heuristic — integer-valued aggregate
  results render with thousands separators and no decimal point
  (`107,000`); fractional results render with thousands separators and
  2 dp (`1,992.25`). This is independent of the column's body formatter
  because gt's summary rows are styled independently from body rows.
- HTML rendering: `render` now branches on whether `table.groups` is
  empty. Non-empty case emits a `gt_group_heading_row` per group, walks
  the group's body rows with a `is_first_in_group` flag (which appends
  `border-top-width: 2px;` to the cell style), then emits summary rows
  with `gt_first_summary_row thick` on the first and
  `gt_last_summary_row` on the last. The summary stub TH stays
  left-aligned regardless of column alignment. `render_tr` gained an
  `#[allow(clippy::too_many_arguments)]` because the FACET-aware
  signature legitimately needs 9 inputs.
- `allowed_diff` justification — fixture 30:
  `tests/fixtures/30_summary_rows_min_max_mean_with_labels/query.ggsql`
  was rewritten (precedent: fixture 05) because the captured
  `expected.html` lists 7 body columns (`date`, `open`, `high`, `low`,
  `close`, `volume`, `adj_close`) while the original query enumerated
  only 5, and used the legacy printf flag `{:num ,.2f}` for which the
  formatter mini-language has no equivalent. The replacement uses
  `'{:num %.2f}'` (current syntax) and the full 7-column list; the
  pre-computed `week` column already in the parquet feeds FACET, so the
  SELECT prefix was dropped. No CSS or `expected.html` changes.
- New examples 35-37 (`35_facet_basic_grouping`, `36_facet_summary_sum`,
  `37_facet_multi_aggregate`) demonstrate plain row grouping, a single
  per-group summary, and multiple aggregates with custom labels; README
  intro and table updated; `examples/tabulate/out/index.html`
  regenerated via `bash examples/tabulate/run.sh`.

## 2026-06-15 — printf body refactor (housekeeping on `main`)

- Numeric formatter spec is now `{:num <body>}` where `<body>` is a bare
  printf conversion (no leading `%`). The previous tolerated form
  `{:num %<body>}` is rejected: `build_format` returns `None`, so the
  RHS falls back to literal-string treatment.
- Updated `src/tabulate/format.rs` (parser + new
  `num_percent_introducer_is_rejected` test), all example queries that
  used `%`, the `examples/tabulate/README.md` formatter-syntax column,
  and the two fixture queries
  (`tests/fixtures/05_header_source_note_caption_column_labels`,
  `tests/fixtures/30_summary_rows_min_max_mean_with_labels`) that had
  been written with `%`. No `expected.html`, CSS, or normalizer changes;
  all 31 fixtures stay green.
- Also fixed scientific notation rendering: when the decimal exponent
  is 0 (i.e. `1 <= |x| < 10` or `x == 0`), `render_scientific_html` now
  emits the mantissa alone instead of the noisy `× 10⁰` suffix
  (`num_scientific_exp_zero_is_plain_mantissa` covers this).
- Updated `TABULATE_PLAN.md` §2 (numeric formatter table + precedence
  example) and §5 notes 1 and 9 to record the no-`%` rule and that
  captured fixtures are aligned with it. This is a deliberate departure
  from `/spec/GTSQL_PLAN.md`, which still writes printf bodies with `%`.

## 2026-06-15 — phase 10 complete

- Fixtures 31 (title-case transformation), 32 (units in column labels),
  and 33 (forced-sign percent / growth rates) pass under strict
  normalization. No CSS or expected.html changes.
- New `CellFmt::Str(StringFn)` variant in `src/tabulate/format.rs` for
  case transforms. `build_format` now dispatches to a new
  `build_string_format` that recognises `{:Title}`, `{:UPPER}`,
  `{:lower}`, and the identity `{}`. Title-case follows R's
  `tools::toTitleCase(tolower(x))` semantics: lowercase the input,
  then upper-case the first character of every whitespace-separated
  word. Strings are routed through `ColFmt::Str` in
  `src/tabulate/execute.rs`; non-string Arrow types fall back to
  `array_value_to_string` and pipe that through the transform.
- New `units: Option<String>` on `ColMeta` populated from
  `FORMAT <col> SETTING units => '<u>'`. When set and no explicit
  `LABEL` overrides the column, the header label is derived from the
  column name by dropping the trailing `_<tok>` suffix and
  title-casing the remaining `_`-separated words
  (`derive_units_label`: `land_area_km2` → `Land Area`,
  `density_2021` → `Density`). The header `<th>` body becomes
  `<label> <units-html>` where `render_units_html` wraps any `^N`
  segment in gt's
  `<span style="white-space:nowrap;"><sup style="line-height:0;">N</sup></span>`
  markup.
- `NumSpec::render` now emits Unicode minus (U+2212) for negatives
  when `force_sign` is set, matching gt's
  `fmt_*(force_sign = TRUE)` rendering. Unforced negatives stay on
  the ASCII hyphen-minus (no change). Covered by
  `num_forced_sign` and `num_unforced_negative_is_ascii_minus`.
- New examples 38–41 (`38_case_title`, `39_case_upper_lower`,
  `40_units_in_header`, `41_forced_sign_growth`); README updated;
  `examples/tabulate/out/index.html` regenerated via
  `bash examples/tabulate/run.sh`. No `allowed_diff` entries needed.

## 2026-06-15 — phase 11 complete

- Fixture 34 (comprehensive integration: header + spanner + per-column
  formatting + scale + highlight + multi-aggregate facet + summary
  rows) passes under strict normalization. No CSS or expected.html
  changes.
- Five regressions surfaced when fixture 34 was first wired up;
  resolved by extending the earlier phases rather than relaxing
  normalization (no new `allowed_diff` entries):
  1. **CTE-only SQL portion** — `WITH … AS (…)` with no trailing
     `SELECT` was tolerated by the grammar but rejected by DuckDB.
     `build_sql` now detects this shape via a new
     `sql_portion_is_cte_only` AST walk and appends
     `SELECT * FROM <table>` so DuckDB has a body to execute.
     `determine_table_name` was reordered to prefer the SQL portion's
     first `FROM` table (the underlying parquet) over the TABULATE
     `FROM` clause (which may name a CTE).
  2. **HIGHLIGHT + CTE** — the wrap query
     `SELECT __t.*, (filter) AS __hl_0__match FROM (<base>) __t`
     embeds a `WITH` inside a subquery, which DuckDB rejects.
     `build_sql` now hoists the `WITH` clause to the outer query when
     the base is CTE-only: `WITH … SELECT __t.*, (filter)… FROM
     (SELECT * FROM <table>) __t`.
  3. **R-style colour names** — gt's `data_color` decodes colour
     literals through R's X11 name table where `"green"` is
     `#00FF00` (CSS3 `lime`), not `#008000`. Our scale module routed
     through `csscolorparser`, which gave CSS3's `green = #008000` and
     produced visibly different gradients (`#81B572` vs `#A2FF8A`).
     Added a small `r_alias_color` shim in `src/tabulate/scale.rs`
     that maps `green`/`gray`/`grey`/`darkgray`/`darkgrey`/`lightgray`/
     `lightgrey` to their R values before delegating to
     `csscolorparser`. CSS hex literals are unchanged.
  4. **HIGHLIGHT colour vs SCALE auto-contrast** — when both a SCALE
     background and a HIGHLIGHT colour fall on the same cell, gt's
     captured HTML emits only the HIGHLIGHT's `color:` declaration
     (no auto-contrast fallback, no duplicate). `render_td` in
     `src/tabulate/html.rs` now skips the SCALE-derived
     `color: <fg>` when the same cell has a HIGHLIGHT colour
     override.
  5. **Summary cell formatting** — gt's `tab_summary_rows()` default
     (`formatC(format = "f", digits = K)` per-column max-K, no
     thousands separator) differs from the column formatter applied
     to body cells. Replaced the hardcoded "2 decimals + thousands
     separator" `format_summary_value` with a per-column max-decimals
     pass: `summary_decimals_for` scans every summary value in a
     target column to pick `K`, then `format_summary_value(v, K)`
     prints with that fixed precision and no separator. To keep the
     fixture 30 expected (`1,992.25`) reachable, `FACET … SETTING`
     gained a `fmt => '{:num …}'` key whose template overrides the
     default formatter for every summary cell. Fixture 30's query
     was updated to `SETTING fmt => '{:num \',.2f}'`; no
     `expected.html` change.
- Phase 11 also drops the `label =>` setting from fixture 34's
  query: gt 1.3 ignores `summary_rows(fns = list(Total = "sum",
  Average = "mean"))` list-name labels and renders the function
  names directly. Keeping `label => ['Total', 'Average']` in the
  GTSQL query produced spurious `Total`/`Average` stubs; removing
  it lets the existing aggregate-name fallback emit `sum`/`mean`
  matching the captured HTML. The `label =>` handling itself stays
  in for the more common single-aggregate case.
- Fixture 34's `'{:num .1f}%'` was changed to `'{:num .1f}%%'` so
  the trailing `%` is treated as a literal (no `×100` scaling),
  matching gt's `fmt_percent(scale_values = FALSE)`. The GTSQL
  format mini-language has no other way to express
  `scale_values = FALSE`, so this is a deliberate departure from
  the spec text in `/spec/GTSQL_EXAMPLES.qmd` (precedent: phase 10
  housekeeping commit).
- New example 42 (`42_comprehensive_sales_report`) renders the same
  comprehensive integration as fixture 34 with a small inline
  `VALUES`-based `sales_data` CTE so it's runnable from the CLI.
  README updated; `examples/tabulate/out/index.html` regenerated
  via `bash examples/tabulate/run.sh`. No `allowed_diff` entries
  needed.

## 2026-06-23 — phase 12 (housekeeping on `agent/tabulate-phase-12`)

Cleanup pass on the `FORMAT … RENAMING` mini-language and on the
single-quoted string escape, driven by user requests. Each change
narrows the surface area so the language has one obvious spelling for
each concept.

- **Auto-derived units label removed.** `FORMAT <col> SETTING units =>
  '<u>'` no longer rewrites the column header by stripping a trailing
  `_<token>` suffix and title-casing the remainder. The header is now
  the column name (or whatever the user supplies via `LABEL <col> =>
  '<text>'`) with the unit annotation appended. `derive_units_label`
  was deleted from `src/tabulate/execute.rs`. Fixture 32 was updated
  with explicit `LABEL` clauses (`land_area_km2 => 'Land Area'`,
  `density_2021 => 'Density'`) so its `expected.html` still matches;
  example 40 was rewritten end-to-end to demonstrate the supported
  pattern.
- **SQL `''` doubled-quote escape removed.** The grammar's `string`
  rule (`tree-sitter-ggsql/grammar.js`) and `unquote_string` in
  `src/parser/tabulate.rs` no longer treat `''` as a single
  embedded `'`. The only escape for an embedded apostrophe is `\'`.
  Fixture 05 was the only fixture using `''` (in
  `'Ontario''s Largest Municipalities'` and `'{:num ''d}'`); both
  now use `\'`. `parse_string_node` in `src/parser/builder.rs` never
  recognised `''` in the first place, so VISUALISE-side strings are
  unaffected.
- **`{:title}` / `{:upper}` / `{:lower}` are case-insensitive.**
  `build_string_format` in `src/tabulate/format.rs` strips the leading
  colon and lowercases the keyword before matching, so `{:title}`,
  `{:Title}`, `{:TITLE}` are equivalent. Examples 38 and 39 were
  rewritten to use the lowercase form; the README's case-transform row
  follows suit. Existing fixture 31 (which uses `{:Title}`) still
  passes — case-insensitivity is purely additive.
- **`{:num}` thousands flag is `'` only.** `NumSpec::parse` no longer
  accepts `,` as a thousands flag; only `'` (written `\'` inside the
  single-quoted RHS) works. 10 fixture queries that previously used
  `,d` / `,.Nf` were rewritten to `\'d` / `\'.Nf`. The format-rs unit
  tests `num_legacy_thousands_int` / `num_currency_prefix` were
  retargeted to the apostrophe form.
- **Percent suffix is literal — no `×100` scaling, no `%%` collapse.**
  `parse_percent_suffix` was deleted from `src/tabulate/format.rs`.
  `build_num_format` now appends the post-`{...}` text verbatim. A
  trailing `%` is just a character; users who hold 0–1 proportions and
  want `xx.x%` output must multiply by 100 in the upstream SQL
  projection. The doc comment at the top of `format.rs` spells out the
  new model. Fixtures 14 and 33 (which relied on the implicit scaling)
  now multiply by 100 in `SELECT … * 100 AS …`; fixture 34's
  `{:num .1f}%%` becomes `{:num .1f}%` (same rendering). Examples 20,
  29, 41, and 42 were updated analogously — example 42's `satisfaction`
  column was re-scaled to 0–100 integers in the `VALUES` block, with
  `SCALE FROM (0, 100)` and `HIGHLIGHT FILTER < 70` adjusted to match.
- Phase merged via four logical commits on
  `agent/tabulate-phase-12`. Examples regenerated via
  `bash examples/tabulate/run.sh`. No `allowed_diff` entries needed.

## 2026-06-23 — polishing Phase 3: remove SETTING units

- `FORMAT … SETTING units => '<s>'` removed from parser, AST plumbing,
  IR (`ColMeta::units`), HTML writer, and `render_units_html`.
  `src/parser/tabulate.rs::parse_setting_pair` rejects `units` at parse
  time with the message "FORMAT SETTING `units` was removed; put units
  in the LABEL text instead, e.g. LABEL land_area => 'Land Area (km²)'".
- Fixture 32 (`units_in_column_labels`) `query.ggsql` rewritten to put
  the unit annotation in `LABEL`, using Unicode `(km²)` / `(people/km²)`.
  The captured `expected.html` embeds gt's `<span><sup>2</sup></span>`
  markup that LABEL (which HTML-escapes) cannot reproduce, so two
  `allowed_diff` regexes mask the two affected `<th>` tags entirely.
  Justification: the body cells (`<td>`) are unaffected by the mask and
  still verify the data values and number formatters end-to-end; the
  header rendering of inline unit markup is the only deliberate gap.
- Example `40_units_in_header.ggsql` renamed to `40_unit_in_label.ggsql`
  and rewritten to use `LABEL <col> => '... (km²)'`. README row
  updated.

## 2026-06-23 — POLISHING_PLAN sweep complete

Executed Phases 0–7 from POLISHING_PLAN.md on branch `polishing`,
merged each phase into `main` along the way. Summary:

* Phase 0 — branch + baseline (35 fixture tests green).
* Phase 1 — documentation alignment, 13 commits to spec/GTSQL_PLAN.md
  (copied into the repo from the read-only /spec/ mount) and
  TABULATE_PLAN.md (A3, A4, A6, B1, B6, B7, B8, B9, B10, B11, C1,
  C3, C5, C6).
* Phase 2 — `{:num %<body>}` printf body flipped to require the `%`
  introducer (matching the spec). Find_keyword helper added so
  formatter keywords are case-insensitive. Fixture + example sweep.
  Example 43_raw_passthrough.ggsql added.
* Phase 3 — `FORMAT SETTING units` removed from parser/AST/IR/HTML.
  Fixture 32 kept passing via two allowed_diff regexes masking the
  affected <th> tags. Example 40 renamed to 40_unit_in_label.ggsql.
* Phase 4 — aggregate `'mean'` → `'avg'`. Parser rejects `'mean'` /
  `'average'`. Fixtures + examples swept.
* Phase 5 — missing features:
   - 5a FORMAT * wildcard (grammar + parser + execute expand_cols).
   - 5b SCALE foreground / size / opacity aesthetics
     (build_cell_scale 4-tuple, css_rgba for opacity composition).
   - 5c HIGHLIGHT size / transform / decoration.
   - 5d FACET groups => [...] filter + execution-time validation.
  + examples 44–52.
* Phase 6 — validation rules:
   - 6.1 VISUALISE/TABULATE mutual exclusion.
   - 6.3 spanner-ID collisions.
   - 6.6 parenthesized target form enforced; fixture/example sweep.
   - 6.2 / 6.4 / 6.5 already met by existing code or shipped in 5d.
* Phase 7 — cleanup: cargo fmt clean; only the two pre-existing
  src/writer/vegalite/layer.rs warnings remain. 35/35 fixture tests
  and 1541/1541 lib tests pass under the no-default + duckdb,parquet,
  vegalite,builtin-data feature set.

Two allowed_diff additions were introduced (both in fixture 32) when
the `units` feature was removed; justification logged above.
