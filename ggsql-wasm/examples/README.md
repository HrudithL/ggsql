# `ggsql-wasm` examples

This folder is self-contained: the 53 TABULATE `.ggsql` scenarios live
under `scenarios/`, and everything you need to render them for the
`ggsql-wasm` browser engine and the Quarto integration that ships in
[`../demo/`](../demo/) is in this folder or the sibling `ggsql-wasm/`
crate. The main artifact — the browser-executable Quarto document —
is [`tabulate.qmd`](tabulate.qmd); the static reviewer-facing render is
[`out/preview.html`](out/preview.html).

## Folder layout

| Path | What it is |
|---|---|
| [`tabulate.qmd`](tabulate.qmd) | **Main artifact.** Quarto source with one `{ggsql}` cell per scenario. Render with the `ggsql-wasm`-aware Quarto setup and every cell executes live in the browser. |
| [`scenarios/`](scenarios/) | Individual `NN_<slug>.ggsql` files (53 total), one per scenario. Read by `build_qmd.py` and `run.sh`. Kept out of the top-level listing to reduce clutter. |
| [`out/preview.html`](out/preview.html) | Static preview produced by the CLI binary (which uses the same `tabulate::html::render` as the WASM engine). Committed, so reviewers can view every scenario without running anything locally. The gt-random ids are normalized by `run.sh` so this file is byte-stable across re-runs. |
| `out/quarto/` | Full Quarto render of `tabulate.qmd`. **Not committed** (Quarto asset hashes and internal JS drift with Quarto's version, which would produce noisy diffs on every render). Produced by `run.sh` when both `quarto` and the wasm bundle are available; open `out/quarto/tabulate.html` locally to see the live in-browser version. |
| [`build_qmd.py`](build_qmd.py) | Regenerates `tabulate.qmd` from `scenarios/NN_<slug>.ggsql`. |
| [`run.sh`](run.sh) | Orchestrates: regenerate `.qmd`, build `preview.html`, and (optionally) run `quarto render`. |

## Prerequisites

From the **repo root**:

1.  Build the CLI binary — the same renderer that runs inside the WASM
    engine uses this for the static preview:
    ```sh
    cargo build -p ggsql-cli
    ```
2.  Build the WASM bundle (needed for the live in-browser render):
    ```sh
    cd ggsql-wasm && ./build-wasm.sh && cd ..
    ```
3.  Install Quarto if you don't already have it: <https://quarto.org>.

## Render every scenario (static preview + Quarto site)

From the repo root:

```sh
./ggsql-wasm/examples/run.sh           # uses target/debug/ggsql
./ggsql-wasm/examples/run.sh --release # uses target/release/ggsql
"$BROWSER" ggsql-wasm/examples/out/preview.html
```

`run.sh` does three things:

1.  Regenerates `tabulate.qmd` from the `scenarios/NN_<slug>.ggsql`
    files via `build_qmd.py`.
2.  Rebuilds the static `out/preview.html` by running every scenario
    through the CLI.
3.  If `quarto` is on `PATH` and the wasm bundle exists at
    `ggsql-wasm/demo/dist/`, also runs `quarto render tabulate.qmd`
    into `out/quarto/`.

Open `out/quarto/tabulate.html` for the fully-rendered Quarto page (which
loads the wasm bundle from `./wasm/` alongside it so each cell is
re-editable in the browser).

## Render live (Quarto + WASM only)

If you already ran the bundle build in Prerequisites step 2:

```sh
quarto render ggsql-wasm/examples/tabulate.qmd
```

The `{ggsql}` cell handler intercepts every block, routes the query
through `executeTable` (when `has_tabulate && !has_visual`), and inlines
the rendered HTML beneath the source. The static preview produced by
`run.sh` is byte-for-byte the same modulo the host page wrapper, because
the WASM and CLI surfaces share `ggsql::tabulate::html::render`.

## In-playground examples

The interactive playground under [`../demo/`](../demo/) already curates
a shorter subset of TABULATE scenarios in its **Tables** section — see
[`../demo/src/examples.ts`](../demo/src/examples.ts). That list is
hand-tuned for the in-page editor; `tabulate.qmd` in this folder is the
canonical full set.

## Add a new scenario

1.  Add `ggsql-wasm/examples/scenarios/<NN>_<slug>.ggsql` (use the next
    available number and a snake_case slug).
2.  From any working directory, re-run:
    ```sh
    ./ggsql-wasm/examples/run.sh
    ```
    The `.qmd`, the static preview, and (if `quarto` is available) the
    Quarto render all refresh from disk. No manual edits required.
3.  Commit the new `scenarios/<NN>_<slug>.ggsql`, the regenerated
    `tabulate.qmd`, and the regenerated `out/preview.html`. Do **not**
    commit `out/quarto/` — it is gitignored because Quarto's asset
    hashes and internal JS drift from version to version.

Files whose name ends in `_error` are negative tests — `run.sh` captures
stderr and inlines the diagnostic in the preview rather than aborting.
