# TABULATE examples — WASM

Every scenario from [`../tabulate/`](../tabulate/) packaged for the
`ggsql-wasm` browser engine and the Quarto integration that ships in
[`ggsql-wasm/demo/`](../../ggsql-wasm/demo/).

There are three artifacts in this folder:

| File | What it is |
|---|---|
| [`tabulate.qmd`](tabulate.qmd) | Quarto source with one `{ggsql}` cell per scenario. Render with the `ggsql-wasm`-aware Quarto setup and every cell executes live in the browser. |
| [`out/preview.html`](out/preview.html) | Static preview produced by the CLI binary (which uses the same `tabulate::html::render` as the WASM engine). Use this as a reviewer-friendly reference when you can't render the `.qmd`. |
| [`build_qmd.py`](build_qmd.py) | Regenerates `tabulate.qmd` from `../tabulate/*.ggsql`. |

## In-playground examples

The interactive playground at [`ggsql-wasm/demo/`](../../ggsql-wasm/demo/)
already curates a shorter subset of TABULATE scenarios in its **Tables**
section — see the `Tables` entries in
[`ggsql-wasm/demo/src/examples.ts`](../../ggsql-wasm/demo/src/examples.ts).
That list is hand-tuned for the in-page editor; the `tabulate.qmd` file
in this folder is the canonical full set.

## Render every scenario (preview)

```sh
./examples/wasm/run.sh           # uses target/debug/ggsql
./examples/wasm/run.sh --release # uses target/release/ggsql
"$BROWSER" examples/wasm/out/preview.html
```

`run.sh` regenerates `tabulate.qmd` from `examples/tabulate/*.ggsql` and
also rebuilds `out/preview.html`.

## Render live (Quarto + WASM)

```sh
# Build the wasm bundle once:
cd ggsql-wasm && ./build-wasm.sh

# Then either preview the bundled playground (which Quarto site loads
# automatically):
cd ggsql-wasm/demo && npm install && npm run dev

# …or render this folder's tabulate.qmd through a Quarto site that
# imports the same bundle. See ggsql-wasm/CLAUDE.md and doc/CLAUDE.md
# for the standalone wiring.
quarto render examples/wasm/tabulate.qmd
```

The `{ggsql}` cell handler intercepts every block, routes the query
through `executeTable` (when `has_tabulate && !has_visual`), and inlines
the rendered HTML beneath the source. The static preview produced by
`run.sh` is byte-for-byte the same modulo the host page wrapper, because
the WASM and CLI surfaces share `ggsql::tabulate::html::render`.

## Add a new scenario

Add the `.ggsql` file to `examples/tabulate/` (the cli surface owns the
canonical scenario set) and re-run `examples/wasm/run.sh`. The `.qmd`
and the preview refresh from disk; no manual edits required.
