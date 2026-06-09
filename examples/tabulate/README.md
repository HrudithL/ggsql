# TABULATE examples

Runnable `.ggsql` examples that exercise the TABULATE surface implemented
so far — phase 1 (column selection / reordering / hide / `*`), phase 2
(`FORMAT STUB`, `LABEL title/subtitle/caption`, per-column header relabels,
and basic `{:num ...}` formatters), and phase 3 (`FORMAT SPAN <cols> AS
<id>` with nesting + `LABEL` through the spanner namespace).

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
| [`04_select_star.ggsql`](04_select_star.ggsql) | `TABULATE *` — explicit wildcard, equivalent to the bare form |
| [`05_sql_filter.ggsql`](05_sql_filter.ggsql)   | `ORDER BY` / `LIMIT` before TABULATE      |
| [`06_stub.ggsql`](06_stub.ggsql)           | `FORMAT STUB <col>` row-label column          |
| [`07_title_subtitle.ggsql`](07_title_subtitle.ggsql) | `LABEL title => …, subtitle => …`   |
| [`08_column_labels.ggsql`](08_column_labels.ggsql) | Per-column header relabel + `caption` source-note |
| [`09_number_format.ggsql`](09_number_format.ggsql) | `RENAMING * => '{:num %\'d}'` thousands separator |
| [`10_full_header.ggsql`](10_full_header.ggsql) | All phase-2 features composed in one query |
| [`11_spanner.ggsql`](11_spanner.ggsql)     | `FORMAT SPAN <cols> AS <id>` single spanner   |
| [`12_two_spanners.ggsql`](12_two_spanners.ggsql) | Two side-by-side spanners + `LABEL <id> => …` |
| [`13_nested_spanners.ggsql`](13_nested_spanners.ggsql) | Nested (stacked) spanners over spanners |
| [`14_full_spanner_report.ggsql`](14_full_spanner_report.ggsql) | Phase 1–3 composed: stub + nested spanners + relabels + formatting + header |
