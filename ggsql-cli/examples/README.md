# `ggsql-cli` examples

Runnable `.ggsql` files that exercise the TABULATE surface through the
`ggsql` command-line binary, plus a helper script that renders them all
to a single HTML page.

## Folder layout

| File / pattern | What it is |
|---|---|
| [`scenarios/`](scenarios/) | The 53 individual `NN_<slug>.ggsql` scenario files. Read by `run.sh`. Kept out of the top-level listing to reduce clutter. |
| [`run.sh`](run.sh) | Renders every `scenarios/*.ggsql` file through `ggsql run` into `out/<name>.html`, plus a single `out/index.html` listing all queries side-by-side with their rendered tables. |
| `out/` (gitignored) | Populated by `run.sh`. Open `out/index.html` in a browser to see every scenario and its rendered HTML table. |

This is the **canonical scenario set**. The other surfaces
(`ggsql-jupyter/examples/scenarios/`, `ggsql-wasm/examples/scenarios/`,
`ggsql-vscode/examples/scenarios/`) all carry the same 53 queries and
replay them through their own surfaces.

## Prerequisites

Build the CLI binary once from the repo root:

```sh
cargo build -p ggsql-cli            # debug build — target/debug/ggsql
cargo build -p ggsql-cli --release  # release build — target/release/ggsql
```

The `run.sh` helper in this folder will build for you if the binary is
missing.

## Cross-references

For the same scenarios in the other surfaces see
[`../../ggsql-jupyter/examples/`](../../ggsql-jupyter/examples/),
[`../../ggsql-wasm/examples/`](../../ggsql-wasm/examples/), and
[`../../ggsql-vscode/examples/`](../../ggsql-vscode/examples/).

Implemented so far — phase 1 (column selection / reordering / hide / `*`),
phase 2
(`FORMAT STUB`, `LABEL title/subtitle/caption`, per-column header relabels,
and basic `{:num ...}` formatters), phase 3 (`FORMAT SPAN <cols> AS
<id>` with nesting + `LABEL` through the spanner namespace), phase 4
(`FORMAT <col> SETTING width / align`), phase 5 (the full
`{:num <printf>}` and `{:time <strftime>}` formatter mini-language plus
per-column `SETTING locale => '...'`), phase 6 (`RENAMING null|0|'literal'
=> '<text>'` direct value substitution), phase 7 (`SCALE background`
continuous colour scales — named palettes, explicit colour stops,
explicit `FROM (lo, hi)` domain, and `VIA log10` transform), and
phase 8 (`HIGHLIGHT <cols> FILTER <SQL predicate> SETTING ...` for
predicate-driven cell-level highlights, including multiple HIGHLIGHTs
in the same query), phase 9 (`FACET <col>` for row grouping with
an optional `SETTING target / aggregate / label / side` block to emit
per-group summary rows), and phase 10 (case-transform mini-language
`{:title}` / `{:upper}` / `{:lower}`, `FORMAT … SETTING units => '<u>'`
in column headers, and the forced-sign acceptance test for
`{:num +.Nf}%`), and phase 11 (an end-to-end integration example that
exercises SQL CTE → header → spanner → per-column formats →
`SCALE background` → `HIGHLIGHT … FILTER` → `FACET … SETTING fmt =>
'<template>'` in a single query).

## Run every example

From the **repo root**:

```sh
./ggsql-cli/examples/run.sh           # uses target/debug/ggsql
./ggsql-cli/examples/run.sh --release # uses target/release/ggsql
```

The script writes one HTML file per query to `ggsql-cli/examples/out/`
and also produces `out/index.html`, a single page listing every example
with its source query and rendered table side-by-side. Open it in a
browser:

```sh
"$BROWSER" ggsql-cli/examples/out/index.html
```

## Run one example

From the repo root:

```sh
./target/debug/ggsql run ggsql-cli/examples/scenarios/01_minimal.ggsql
```

Add `--output path.html` to write to a file instead of stdout, or pipe to
a browser:

```sh
./target/debug/ggsql run ggsql-cli/examples/scenarios/01_minimal.ggsql > /tmp/t.html
"$BROWSER" /tmp/t.html
```

## Adding a new example

1. Create `ggsql-cli/examples/scenarios/<NN>_<slug>.ggsql`. Use the
   next available number and a short snake_case slug.
2. Re-run `./ggsql-cli/examples/run.sh` to refresh `out/`.
3. If the scenario should also appear on the other surfaces, copy the
   file into their `scenarios/` folders and regenerate their
   artifacts:
   - `python3 ggsql-vscode/examples/build_ggsql.py` (multi-cell view)
   - `python3 ggsql-jupyter/examples/build_notebook.py` (Jupyter notebook)
   - `python3 ggsql-wasm/examples/build_qmd.py` (Quarto document)
4. Files whose name ends in `_error` are negative tests — `run.sh`
   captures stderr and embeds it in the index instead of aborting.

## Files

| File                                       | Demonstrates                                  |
| ------------------------------------------ | --------------------------------------------- |
| [`01_minimal.ggsql`](scenarios/01_minimal.ggsql)     | Bare `TABULATE` — every column from the SELECT |
| [`02_reorder.ggsql`](scenarios/02_reorder.ggsql)     | Column selection / reordering via an explicit TABULATE list |
| [`03_hide.ggsql`](scenarios/03_hide.ggsql)           | `FORMAT col SETTING hide => true`             |
| [`04_sql_filter.ggsql`](scenarios/04_sql_filter.ggsql)   | `ORDER BY` / `LIMIT` before TABULATE      |
| [`05_stub.ggsql`](scenarios/05_stub.ggsql)           | `FORMAT STUB <col>` row-label column          |
| [`06_title_subtitle.ggsql`](scenarios/06_title_subtitle.ggsql) | `LABEL title => …, subtitle => …`   |
| [`07_column_labels.ggsql`](scenarios/07_column_labels.ggsql) | Per-column header relabel + `caption` source-note |
| [`08_number_format.ggsql`](scenarios/08_number_format.ggsql) | `RENAMING * => '{:num \'d}'` thousands separator |
| [`09_full_header.ggsql`](scenarios/09_full_header.ggsql) | All phase-2 features composed in one query |
| [`10_spanner.ggsql`](scenarios/10_spanner.ggsql)     | `FORMAT SPAN <cols> AS <id>` single spanner   |
| [`11_two_spanners.ggsql`](scenarios/11_two_spanners.ggsql) | Two side-by-side spanners + `LABEL <id> => …` |
| [`12_nested_spanners.ggsql`](scenarios/12_nested_spanners.ggsql) | Nested (stacked) spanners over spanners |
| [`13_full_spanner_report.ggsql`](scenarios/13_full_spanner_report.ggsql) | Phase 1–3 composed: stub + nested spanners + relabels + formatting + header |
| [`14_widths_align.ggsql`](scenarios/14_widths_align.ggsql) | `FORMAT col SETTING width => …, align => …` |
| [`15_align_override.ggsql`](scenarios/15_align_override.ggsql) | `SETTING align => …` overriding the auto-aligned default |
| [`16_widths_with_spanner.ggsql`](scenarios/16_widths_with_spanner.ggsql) | `SETTING width => …` composed with `FORMAT SPAN …` |
| [`17_num_decimals.ggsql`](scenarios/17_num_decimals.ggsql) | `{:num .3f}` — fixed decimal places |
| [`18_num_thousands.ggsql`](scenarios/18_num_thousands.ggsql) | `{:num \'d}` — integer with thousands separators |
| [`19_currency.ggsql`](scenarios/19_currency.ggsql) | `${:num \'d}` — literal currency prefix with separators |
| [`20_percent.ggsql`](scenarios/20_percent.ggsql) | `{:num .1f}%` — trailing `%` is a literal suffix; pre-scale 0-1 data in SQL |
| [`21_scientific.ggsql`](scenarios/21_scientific.ggsql) | `{:num .2e}` — scientific notation with HTML `<sup>` exponent |
| [`22_dates.ggsql`](scenarios/22_dates.ggsql) | `{:time %B %-d, %Y}` — date formatting with strftime directives |
| [`23_datetime.ggsql`](scenarios/23_datetime.ggsql) | Mixed date / time / datetime columns with `{:time ...}` |
| [`24_french_locale.ggsql`](scenarios/24_french_locale.ggsql) | `SETTING locale => 'fr'` for French month and weekday names |
| [`25_replace_missing.ggsql`](scenarios/25_replace_missing.ggsql) | `RENAMING null => '<text>'` — substitute missing values (`---` becomes em-dash) |
| [`26_replace_zero.ggsql`](scenarios/26_replace_zero.ggsql) | `RENAMING 0 => '<text>'` — substitute zero cells, composed with `* => '{:num \'d}'` |
| [`27_direct_value_mapping.ggsql`](scenarios/27_direct_value_mapping.ggsql) | `RENAMING '<value>' => '<text>'` — exact-value lookup table |
| [`28_scale_named_palette.ggsql`](scenarios/28_scale_named_palette.ggsql) | `SCALE background TO viridis` — colour cells along a named gt palette |
| [`29_scale_explicit_colors.ggsql`](scenarios/29_scale_explicit_colors.ggsql) | `SCALE background TO ('<color>', '<color>')` — two-stop gradient in Lab space |
| [`30_scale_explicit_domain.ggsql`](scenarios/30_scale_explicit_domain.ggsql) | `SCALE background FROM (lo, hi) TO RdYlGn SETTING target => (col, col, …)` — fixed domain across several columns |
| [`31_scale_log_transform.ggsql`](scenarios/31_scale_log_transform.ggsql) | `SCALE background … VIA log10` — log-warped colour mapping for wide-range data |
| [`32_highlight_failing_scores.ggsql`](scenarios/32_highlight_failing_scores.ggsql) | `HIGHLIGHT score FILTER score < 60 SETTING face => 'bold', color => 'red'` — single-column conditional cell highlight |
| [`33_highlight_region_row.ggsql`](scenarios/33_highlight_region_row.ggsql) | `HIGHLIGHT revenue, units, margin FILTER region = 'West' SETTING background => '#fff3cd'` — multi-column highlight on rows matching a predicate |
| [`34_highlight_up_down_days.ggsql`](scenarios/34_highlight_up_down_days.ggsql) | Two `HIGHLIGHT` clauses (up-day green, down-day red) composed with currency formatting |
| [`35_facet_basic_grouping.ggsql`](scenarios/35_facet_basic_grouping.ggsql) | `FACET category` — group body rows by a column with a heading row per group |
| [`36_facet_summary_sum.ggsql`](scenarios/36_facet_summary_sum.ggsql) | `FACET … SETTING target => (cols), aggregate => ('sum')` — one summary row per group |
| [`37_facet_multi_aggregate.ggsql`](scenarios/37_facet_multi_aggregate.ggsql) | Multiple aggregates (`min`, `max`, `mean`) with custom `label` overrides |
| [`38_case_title.ggsql`](scenarios/38_case_title.ggsql) | `RENAMING * => '{:title}'` — title-case each cell (first letter of every word) |
| [`39_case_upper_lower.ggsql`](scenarios/39_case_upper_lower.ggsql) | `{:upper}` and `{:lower}` — normalise text to a single case |
| [`40_unit_in_label.ggsql`](scenarios/40_unit_in_label.ggsql) | Inline `^N` / `_N` super/subscript markup in `LABEL <col> => '... km^2'`. No separate `units` SETTING — anything beyond the column name belongs in LABEL. |
| [`41_forced_sign_growth.ggsql`](scenarios/41_forced_sign_growth.ggsql) | `{:num +.1f}%` — forced-sign percent (positives `+`, negatives Unicode `−`) |
| [`42_comprehensive_report.ggsql`](scenarios/42_comprehensive_report.ggsql) | Integration: SQL CTE → header + spanner + per-column formats + SCALE + HIGHLIGHT + FACET summary, end-to-end |
| [`43_raw_passthrough.ggsql`](scenarios/43_raw_passthrough.ggsql) | `RENAMING * => '${} USD'` — raw `{}` passthrough with literal prefix/suffix; no formatter applied |
| [`44_facet_groups_restrict.ggsql`](scenarios/44_facet_groups_restrict.ggsql) | `FACET … SETTING groups => ['North', 'South']` — restrict summary rows to specific group values |
| [`45_facet_groups_error.ggsql`](scenarios/45_facet_groups_error.ggsql) | **Negative test** — naming a non-existent group in `groups => [...]` errors at execute time with `FACET groups: '<name>' is not a value of grouping column '<col>'`. Files ending in `_error` are handled specially by `run.sh`, which captures the diagnostic and embeds it in the index. |
| [`46_highlight_size.ggsql`](scenarios/46_highlight_size.ggsql) | `HIGHLIGHT … SETTING size => '20px'` — bump the cell `font-size` when the filter matches |
| [`47_highlight_transform.ggsql`](scenarios/47_highlight_transform.ggsql) | `HIGHLIGHT … SETTING transform => 'uppercase'` — apply CSS `text-transform` to matching cells |
| [`48_highlight_decoration.ggsql`](scenarios/48_highlight_decoration.ggsql) | `HIGHLIGHT … SETTING decoration => 'line-through'` — apply CSS `text-decoration` to matching cells |
| [`49_scale_foreground.ggsql`](scenarios/49_scale_foreground.ggsql) | `SCALE foreground FROM (lo, hi) TO ('<lo>', '<hi>')` — continuous text-colour ramp |
| [`50_scale_size.ggsql`](scenarios/50_scale_size.ggsql) | `SCALE size FROM (lo, hi) TO ('12px', '28px')` — continuous font-size ramp |
| [`51_scale_opacity.ggsql`](scenarios/51_scale_opacity.ggsql) | `SCALE opacity FROM (lo, hi) TO ('0.2', '1.0')` — modulates the alpha on a composed `SCALE background` (renders as `rgba(...)`) |
| [`52_format_wildcard.ggsql`](scenarios/52_format_wildcard.ggsql) | `FORMAT * RENAMING null => '---'` — wildcard `*` applies the clause to every visible column; `---` is converted to an em-dash by gt's smart-text processor (table-wide null substitution) |
| [`53_label_markup.ggsql`](scenarios/53_label_markup.ggsql) | LABEL text supports inline markup: `^N` / `_N` for super/subscript, braced `^{...}` / `_{...}` for arbitrary content, and gt's smart-text `---` / `--` / `...` substitutions |
