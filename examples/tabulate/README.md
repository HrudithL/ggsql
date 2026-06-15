# TABULATE examples

Runnable `.ggsql` examples that exercise the TABULATE surface implemented
so far — phase 1 (column selection / reordering / hide / `*`), phase 2
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
`{:Title}` / `{:UPPER}` / `{:lower}`, `FORMAT … SETTING units => '<u>'`
in column headers, and the forced-sign acceptance test for
`{:num +.Nf}%`).

## Run all examples

```sh
./examples/tabulate/run.sh           # uses target/debug/ggsql
./examples/tabulate/run.sh --release # uses target/release/ggsql
```

The script writes one HTML file per query to `examples/tabulate/out/` and
produces `out/index.html` that lists every example with its source query
and rendered table side-by-side. Open it in a browser:

```sh
"$BROWSER" examples/tabulate/out/index.html
```

## Run one example

```sh
./target/debug/ggsql run examples/tabulate/01_minimal.ggsql
```

Add `--output path.html` to write to a file instead of stdout, or pipe to
a browser:

```sh
./target/debug/ggsql run examples/tabulate/01_minimal.ggsql > /tmp/t.html
"$BROWSER" /tmp/t.html
```

## Files

| File                                       | Demonstrates                                  |
| ------------------------------------------ | --------------------------------------------- |
| [`01_minimal.ggsql`](01_minimal.ggsql)     | Bare `TABULATE` — every column from the SELECT |
| [`02_reorder.ggsql`](02_reorder.ggsql)     | Column selection / reordering via an explicit TABULATE list |
| [`03_hide.ggsql`](03_hide.ggsql)           | `FORMAT col SETTING hide => true`             |
| [`04_sql_filter.ggsql`](04_sql_filter.ggsql)   | `ORDER BY` / `LIMIT` before TABULATE      |
| [`05_stub.ggsql`](05_stub.ggsql)           | `FORMAT STUB <col>` row-label column          |
| [`06_title_subtitle.ggsql`](06_title_subtitle.ggsql) | `LABEL title => …, subtitle => …`   |
| [`07_column_labels.ggsql`](07_column_labels.ggsql) | Per-column header relabel + `caption` source-note |
| [`08_number_format.ggsql`](08_number_format.ggsql) | `RENAMING * => '{:num \'d}'` thousands separator |
| [`09_full_header.ggsql`](09_full_header.ggsql) | All phase-2 features composed in one query |
| [`10_spanner.ggsql`](10_spanner.ggsql)     | `FORMAT SPAN <cols> AS <id>` single spanner   |
| [`11_two_spanners.ggsql`](11_two_spanners.ggsql) | Two side-by-side spanners + `LABEL <id> => …` |
| [`12_nested_spanners.ggsql`](12_nested_spanners.ggsql) | Nested (stacked) spanners over spanners |
| [`13_full_spanner_report.ggsql`](13_full_spanner_report.ggsql) | Phase 1–3 composed: stub + nested spanners + relabels + formatting + header |
| [`14_widths_align.ggsql`](14_widths_align.ggsql) | `FORMAT col SETTING width => …, align => …` |
| [`15_align_override.ggsql`](15_align_override.ggsql) | `SETTING align => …` overriding the auto-aligned default |
| [`16_widths_with_spanner.ggsql`](16_widths_with_spanner.ggsql) | `SETTING width => …` composed with `FORMAT SPAN …` |
| [`17_num_decimals.ggsql`](17_num_decimals.ggsql) | `{:num .3f}` — fixed decimal places |
| [`18_num_thousands.ggsql`](18_num_thousands.ggsql) | `{:num \'d}` — integer with thousands separators |
| [`19_currency.ggsql`](19_currency.ggsql) | `${:num \'d}` — literal currency prefix with separators |
| [`20_percent.ggsql`](20_percent.ggsql) | `{:num .1f}%` — trailing `%` multiplies the value by 100 |
| [`21_scientific.ggsql`](21_scientific.ggsql) | `{:num .2e}` — scientific notation with HTML `<sup>` exponent |
| [`22_dates.ggsql`](22_dates.ggsql) | `{:time %B %-d, %Y}` — date formatting with strftime directives |
| [`23_datetime.ggsql`](23_datetime.ggsql) | Mixed date / time / datetime columns with `{:time ...}` |
| [`24_french_locale.ggsql`](24_french_locale.ggsql) | `SETTING locale => 'fr'` for French month and weekday names |
| [`25_replace_missing.ggsql`](25_replace_missing.ggsql) | `RENAMING null => '<text>'` — substitute missing values (`---` becomes em-dash) |
| [`26_replace_zero.ggsql`](26_replace_zero.ggsql) | `RENAMING 0 => '<text>'` — substitute zero cells, composed with `* => '{:num \'d}'` |
| [`27_direct_value_mapping.ggsql`](27_direct_value_mapping.ggsql) | `RENAMING '<value>' => '<text>'` — exact-value lookup table |
| [`28_scale_named_palette.ggsql`](28_scale_named_palette.ggsql) | `SCALE background TO viridis` — colour cells along a named gt palette |
| [`29_scale_explicit_colors.ggsql`](29_scale_explicit_colors.ggsql) | `SCALE background TO ('<color>', '<color>')` — two-stop gradient in Lab space |
| [`30_scale_explicit_domain.ggsql`](30_scale_explicit_domain.ggsql) | `SCALE background FROM (lo, hi) TO RdYlGn SETTING target => (col, col, …)` — fixed domain across several columns |
| [`31_scale_log_transform.ggsql`](31_scale_log_transform.ggsql) | `SCALE background … VIA log10` — log-warped colour mapping for wide-range data |
| [`32_highlight_failing_scores.ggsql`](32_highlight_failing_scores.ggsql) | `HIGHLIGHT score FILTER score < 60 SETTING face => 'bold', color => 'red'` — single-column conditional cell highlight |
| [`33_highlight_region_row.ggsql`](33_highlight_region_row.ggsql) | `HIGHLIGHT revenue, units, margin FILTER region = 'West' SETTING background => '#fff3cd'` — multi-column highlight on rows matching a predicate |
| [`34_highlight_up_down_days.ggsql`](34_highlight_up_down_days.ggsql) | Two `HIGHLIGHT` clauses (up-day green, down-day red) composed with currency formatting |
| [`35_facet_basic_grouping.ggsql`](35_facet_basic_grouping.ggsql) | `FACET category` — group body rows by a column with a heading row per group |
| [`36_facet_summary_sum.ggsql`](36_facet_summary_sum.ggsql) | `FACET … SETTING target => (cols), aggregate => ('sum')` — one summary row per group |
| [`37_facet_multi_aggregate.ggsql`](37_facet_multi_aggregate.ggsql) | Multiple aggregates (`min`, `max`, `mean`) with custom `label` overrides |
| [`38_case_title.ggsql`](38_case_title.ggsql) | `RENAMING * => '{:Title}'` — title-case each cell (first letter of every word) |
| [`39_case_upper_lower.ggsql`](39_case_upper_lower.ggsql) | `{:UPPER}` and `{:lower}` — normalise text to a single case |
| [`40_units_in_header.ggsql`](40_units_in_header.ggsql) | `SETTING units => 'km^2'` — unit annotation in the column header (with `^N` superscript) |
| [`41_forced_sign_growth.ggsql`](41_forced_sign_growth.ggsql) | `{:num +.1f}%` — forced-sign percent (positives `+`, negatives Unicode `−`) |
