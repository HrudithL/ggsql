# `ggsql-jupyter` examples

This folder is fully self-contained: the 53 TABULATE `.ggsql` scenarios
live here directly, and everything you need to run them through the
`ggsql` Jupyter kernel is in this folder or the sibling `ggsql-jupyter/`
crate. The notebook source is [`tabulate.ipynb`](tabulate.ipynb)
(committed with cleared outputs so diffs stay small); the
fully-executed render is [`out/index.html`](out/index.html).

The kernel routes every cell through the exact same
`ggsql::tabulate::execute` and `ggsql::tabulate::html::render` code path
the CLI (`ggsql run`) uses, so the rendered HTML matches the CLI's
render byte-for-byte except for the Jupyter host wrapper.

## Folder layout

| File / pattern | What it is |
|---|---|
| `NN_<slug>.ggsql` (53 files) | Individual runnable TABULATE scenarios. Each has a leading `--` comment describing what it demonstrates. Read directly by `build_notebook.py`. |
| [`tabulate.ipynb`](tabulate.ipynb) | Notebook produced from those scenarios (one markdown header + one code cell per scenario). |
| [`build_notebook.py`](build_notebook.py) | Regenerates `tabulate.ipynb` from the sibling `NN_<slug>.ggsql` files. |
| [`run.sh`](run.sh) | End-to-end: regenerate notebook → execute cells through the kernel → export to `out/index.html` → re-clear source notebook. |
| [`out/index.html`](out/index.html) | Fully-executed HTML render, committed for reviewers. |

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

1. Regenerates `tabulate.ipynb` from the sibling `NN_<slug>.ggsql`
   files via `build_notebook.py` (one markdown-header + one code cell
   per scenario).
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

1.  Add `ggsql-jupyter/examples/<NN>_<slug>.ggsql` (use the next
    available number and a snake_case slug).
2.  From any working directory, re-run:
    ```sh
    ./ggsql-jupyter/examples/run.sh
    ```
    This regenerates `tabulate.ipynb` and `out/index.html` from disk;
    no manual notebook edits are needed.
3.  Commit the new `NN_<slug>.ggsql`, the regenerated `tabulate.ipynb`,
    and `out/index.html`.

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
