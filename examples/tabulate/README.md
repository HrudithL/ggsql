# TABULATE examples

Runnable `.ggsql` examples that exercise the TABULATE surface implemented
so far — phase 1 (column selection / reordering / hide / `*`), phase 2
(`FORMAT STUB`, `LABEL title/subtitle/caption`, per-column header relabels,
and basic `{:num ...}` formatters), phase 3 (`FORMAT SPAN <cols> AS
<id>` with nesting + `LABEL` through the spanner namespace), and phase 4
(`FORMAT <col> SETTING width / align`).

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
| [`08_number_format.ggsql`](08_number_format.ggsql) | `RENAMING * => '{:num %\'d}'` thousands separator |
| [`09_full_header.ggsql`](09_full_header.ggsql) | All phase-2 features composed in one query |
| [`10_spanner.ggsql`](10_spanner.ggsql)     | `FORMAT SPAN <cols> AS <id>` single spanner   |
| [`11_two_spanners.ggsql`](11_two_spanners.ggsql) | Two side-by-side spanners + `LABEL <id> => …` |
| [`12_nested_spanners.ggsql`](12_nested_spanners.ggsql) | Nested (stacked) spanners over spanners |
| [`13_full_spanner_report.ggsql`](13_full_spanner_report.ggsql) | Phase 1–3 composed: stub + nested spanners + relabels + formatting + header |
| [`14_widths_align.ggsql`](14_widths_align.ggsql) | `FORMAT col SETTING width => …, align => …` |
