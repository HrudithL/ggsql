# TABULATE examples

A small set of runnable `.ggsql` examples that exercise the Phase 1
`TABULATE` surface (column selection, reordering, `FORMAT col SETTING
hide => true`, `TABULATE *`, and SQL composition).

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
| [`01_minimal.ggsql`](01_minimal.ggsql)     | Plain `TABULATE col, col, col`                |
| [`02_reorder.ggsql`](02_reorder.ggsql)     | Column reordering via the TABULATE list       |
| [`03_hide.ggsql`](03_hide.ggsql)           | `FORMAT col SETTING hide => true`             |
| [`04_select_star.ggsql`](04_select_star.ggsql) | `TABULATE *` showing every SELECT column  |
| [`05_sql_filter.ggsql`](05_sql_filter.ggsql)   | `ORDER BY` / `LIMIT` before TABULATE      |
