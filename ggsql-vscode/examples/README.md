# `ggsql-vscode` examples

Everything in this folder is designed to be opened and run with the
`ggsql` VS Code / Positron extension (from `ggsql-vscode/`). Two
independent example sets live here:

| File | What it demonstrates |
|---|---|
| [`sample.gsql`](sample.gsql) | Syntax-highlighting showcase — a walk-through of `VISUALISE` / `DRAW` / `SCALE` / `LABEL` / `FACET` for the plotting surface. Not executed; open it to see the grammar highlighted. |
| [`tabulate.ggsql`](tabulate.ggsql) | 53 executable `TABULATE` scenarios packaged as cells (`-- %%` delimiters). Every cell has a **▶ Run Cell** code lens; each renders an inline HTML table when run. Generated from `examples/tabulate/` at the repo root — see [Regenerate](#regenerate). |
| [`tabulate.code-workspace`](tabulate.code-workspace) | VS Code multi-root workspace that opens this folder alongside the repo root and recommends the `posit.ggsql-vscode` extension. |
| [`build_ggsql.py`](build_ggsql.py) | Regenerates `tabulate.ggsql` from `examples/tabulate/*.ggsql`. Committed alongside `tabulate.ggsql`. |

## Prerequisites

1.  A working `ggsql` VS Code or Positron extension. From the repo root:
    ```sh
    cd ggsql-vscode
    npm install
    npm run package             # produces ggsql-<version>.vsix
    ```
    Then in VS Code / Positron, open the Extensions view, click the
    **…** menu, choose *Install from VSIX…*, and pick the generated
    `.vsix`.
2.  A `ggsql-jupyter` kernel installed and discoverable (the extension
    forwards cell runs to this kernel):
    ```sh
    # from the repo root
    cargo build -p ggsql-jupyter
    target/debug/ggsql-jupyter --install --user
    jupyter kernelspec list     # should list `ggsql`
    ```

## Run the syntax showcase

```sh
code   ggsql-vscode/examples/sample.gsql
# or:
positron ggsql-vscode/examples/sample.gsql
```

You should see the ggsql grammar highlighted: SQL clauses, `VISUALISE`,
`DRAW`, `SCALE`, `LABEL`, `FACET`, etc. This file is a visual reference —
running it requires the referenced CSV files, which are not shipped
here.

## Run the TABULATE scenarios

The recommended way is to open the workspace file so both this folder
and the repo root are visible in the file explorer:

```sh
code   ggsql-vscode/examples/tabulate.code-workspace
# or:
positron ggsql-vscode/examples/tabulate.code-workspace
```

VS Code / Positron will prompt to install the recommended
`posit.ggsql-vscode` extension — accept it (or install the `.vsix`
manually as described in [Prerequisites](#prerequisites)).

Then open `tabulate.ggsql`. Every cell is bracketed by `-- %% <slug>`
delimiters and gets its own **▶ Run Cell** code lens.

| Demo | What to do | What you should see |
|---|---|---|
| **Run one cell** | Click the ▶ code lens above `-- %% 01_minimal`. | An inline HTML table appears beneath the cell (Positron) or in the integrated terminal (plain VS Code). |
| **Run every cell** | Right-click anywhere in the file → *Run All Cells*. | All 53 scenarios execute sequentially in the kernel; each renders inline. |
| **Switch connection** | Open the *Connections* pane and pick a different reader instead of the default `duckdb://memory`. | The next ▶ Run Cell executes against the new connection with no source edit. |
| **Negative test** | Run the `45_facet_groups_error` cell. | The cell reports a parse-time error inline — confirms the error path routes back through the kernel. |

If ▶ *Run Cell* does not appear, verify:

- The file's language shows as **ggsql** in the status bar. If not, run
  *Change Language Mode* → **ggsql**.
- `jupyter kernelspec list` reports `ggsql`. If not, re-run
  `target/debug/ggsql-jupyter --install --user`.
- The extension is enabled (Extensions view → search *ggsql*).

## Regenerate `tabulate.ggsql`

`tabulate.ggsql` is a mechanical concatenation of every file in
`examples/tabulate/` at the repo root. To add or edit scenarios:

1.  Edit `examples/tabulate/<slug>.ggsql` at the repo root.
2.  Regenerate from the repo root:
    ```sh
    python3 ggsql-vscode/examples/build_ggsql.py
    ```
3.  Commit `ggsql-vscode/examples/tabulate.ggsql` alongside the upstream
    `.ggsql` edit.

## How it works end-to-end

The extension is a Positron-aware wrapper around the `ggsql-jupyter`
kernel. When a cell runs:

1.  `cellParser.ts` (in `ggsql-vscode/src/`) extracts the cell text
    between two `-- %%` markers.
2.  The text is sent to the registered ggsql language runtime, which
    forwards it to the `ggsql-jupyter` kernel as an `execute_request`.
3.  The kernel calls `ggsql::tabulate::execute::execute_with_reader` and
    `ggsql::tabulate::html::render`, exactly as the CLI (`ggsql run`)
    does.
4.  The resulting `text/html` payload is published back through the
    Jupyter protocol; the extension forwards it to Positron's output
    pane (inline, not the Plots pane).

So a scenario that renders correctly in the CLI or in the Jupyter
notebook (`ggsql-jupyter/examples/`) will render correctly here.
