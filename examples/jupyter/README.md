# TABULATE examples — Jupyter

Every scenario from [`../tabulate/`](../tabulate/) running end-to-end
through the `ggsql` Jupyter kernel. The notebook source lives at
[`tabulate.ipynb`](tabulate.ipynb) (committed with cleared outputs); the
executed render is [`out/index.html`](out/index.html).

The kernel goes through the exact same `ggsql::tabulate::execute` +
`ggsql::tabulate::html::render` path the CLI uses, so the rendered HTML
matches the CLI's `out/` byte-for-byte except for the Jupyter HTML host
wrapper.

## Set up the kernel (one-time)

```sh
cd "$(git rev-parse --show-toplevel)"
pip install jupyter nbformat nbclient nbconvert
cargo build -p ggsql-jupyter
target/debug/ggsql-jupyter --install --user
jupyter kernelspec list   # should now list `ggsql`
```

## Render every scenario

```sh
./examples/jupyter/run.sh           # debug build
./examples/jupyter/run.sh --release # release build
"$BROWSER" examples/jupyter/out/index.html
```

`run.sh` does four things:

1. Regenerates `tabulate.ipynb` from `examples/tabulate/*.ggsql` via
   `build_notebook.py` (one markdown header + one code cell per scenario).
2. Executes every cell through the `ggsql` kernel with nbconvert.
3. Exports the executed notebook to `out/index.html`.
4. Re-clears outputs in `tabulate.ipynb` so the committed source stays
   minimal.

## Run interactively

Open the notebook in JupyterLab, classic Jupyter, VS Code, or Positron
with the `ggsql` kernel selected:

```sh
jupyter lab examples/jupyter/tabulate.ipynb
```

## Add a new scenario

Add the `.ggsql` file to `examples/tabulate/` (the cli surface owns the
canonical scenario set) and re-run `examples/jupyter/run.sh`. The
notebook and HTML refresh from disk; no manual edits required.

Files whose name ends in `_error` are negative tests — the cell gets the
`raises-exception` tag so nbconvert keeps going and the error appears as
the cell output.
