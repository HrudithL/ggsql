# `ggsql-jupyter` examples

Every `TABULATE` scenario from `ggsql-cli/examples/` running end-to-end
through the `ggsql` Jupyter kernel implemented in this crate. The
notebook source is [`tabulate.ipynb`](tabulate.ipynb) (committed with
cleared outputs so diffs stay small); the fully-executed render is
[`out/index.html`](out/index.html).

The kernel routes every cell through the exact same
`ggsql::tabulate::execute` and `ggsql::tabulate::html::render` code path
the CLI (`ggsql run`) uses, so the rendered HTML matches the CLI's
`ggsql-cli/examples/out/` byte-for-byte except for the Jupyter host
wrapper.

## Prerequisites

Do these three things once, from the **repo root**:

```sh
# 1. Python side — nbformat/nbclient/nbconvert are needed by run.sh
pip install jupyter nbformat nbclient nbconvert

# 2. Build the kernel binary
cargo build -p ggsql-jupyter            # debug   -> target/debug/ggsql-jupyter
# or:
cargo build -p ggsql-jupyter --release  # release -> target/release/ggsql-jupyter

# 3. Register the kernel with Jupyter
target/debug/ggsql-jupyter --install --user
jupyter kernelspec list                 # should list `ggsql`
```

## Render every scenario in one shot

From the repo root:

```sh
./ggsql-jupyter/examples/run.sh           # uses target/debug/ggsql-jupyter
./ggsql-jupyter/examples/run.sh --release # uses target/release/ggsql-jupyter
"$BROWSER" ggsql-jupyter/examples/out/index.html
```

`run.sh` does four things in order:

1. Regenerates `tabulate.ipynb` from the `.ggsql` files under
   `examples/tabulate/` at the repo root via `build_notebook.py`
   (one markdown-header + one code cell per scenario).
2. Executes every cell through the `ggsql` kernel using
   `jupyter nbconvert --execute`.
3. Exports the executed notebook to `out/index.html`.
4. Re-clears outputs in `tabulate.ipynb` so the committed notebook stays
   minimal and diffs stay readable.

## Run interactively

Open the notebook in JupyterLab, classic Jupyter, VS Code, or Positron
and pick the `ggsql` kernel:

```sh
jupyter lab ggsql-jupyter/examples/tabulate.ipynb
```

Execute cells one at a time (`Shift+Enter`) — each `TABULATE` query
renders an HTML table inline underneath the code cell.

## Add a new scenario

The `.ggsql` scenario set is owned by the CLI surface. To add a scenario:

1.  Add or edit `examples/tabulate/<NN>_<slug>.ggsql` at the repo root
    (mirrored by `ggsql-cli/examples/` once the CLI surface has merged).
2.  From the repo root, re-run:
    ```sh
    ./ggsql-jupyter/examples/run.sh
    ```
    This regenerates `tabulate.ipynb` and `out/index.html` from disk;
    no manual notebook edits are needed.
3.  Commit `tabulate.ipynb` and `out/index.html` alongside the
    scenario source.

Files whose name ends in `_error` are negative tests — the corresponding
notebook cell is tagged `raises-exception`, so `nbconvert --execute`
keeps going and the error is captured as the cell's output.

## Troubleshooting

- **`jupyter: command not found`** — you skipped step 1 of the
  Prerequisites. Install with `pip install jupyter nbformat nbclient nbconvert`.
- **`No kernel named 'ggsql'`** — the kernel is built but not registered.
  Re-run `target/debug/ggsql-jupyter --install --user` and confirm with
  `jupyter kernelspec list`.
- **Cells hang** — the kernel probably crashed on the first cell.
  Re-run with `cargo build -p ggsql-jupyter --release && ./ggsql-jupyter/examples/run.sh --release`
  and check `target/release/ggsql-jupyter --help` starts cleanly.
