# `ggsql-vscode` examples

Everything in this folder is designed to be opened and run with the
`ggsql` VS Code / Positron extension (from `ggsql-vscode/`). The
folder is self-contained — the 53 TABULATE scenarios live under
`scenarios/`, and the primary artifact — the multi-cell runnable file
that lights up the extension's ▶ Run Cell code lens — sits at the
folder root.

| File / pattern | What it is |
|---|---|
| [`sample.gsql`](sample.gsql) | Syntax-highlighting showcase — a walk-through of `VISUALISE` / `DRAW` / `SCALE` / `LABEL` / `FACET` for the plotting surface. Not executed; open it to see the grammar highlighted. |
| [`tabulate.ggsql`](tabulate.ggsql) | **Main artifact.** All 53 TABULATE scenarios concatenated into one file with `-- %%` cell delimiters, so every scenario gets its own **▶ Run Cell** code lens in the extension. Generated from `scenarios/` by `build_ggsql.py` — see [Regenerate](#regenerate). |
| [`scenarios/`](scenarios/) | Individual `NN_<slug>.ggsql` files (53 total), one per scenario. Kept out of the top-level listing to reduce clutter. Each file has a leading `--` comment describing what it demonstrates. |
| [`tabulate.code-workspace`](tabulate.code-workspace) | VS Code multi-root workspace that opens this folder alongside the repo root and recommends the `ggsql.ggsql` extension. |
| [`build_ggsql.py`](build_ggsql.py) | Regenerates `tabulate.ggsql` from the `scenarios/NN_<slug>.ggsql` files. Committed alongside `tabulate.ggsql`. |

## Prerequisites

1.  A working `ggsql` VS Code or Positron extension, built and installed
    from source. From the repo root:
    ```sh
    cd ggsql-vscode
    npm install
    npm run package                       # bundles TS -> out/extension.js
    npx @vscode/vsce package              # produces ggsql-<version>.vsix
    code --install-extension ggsql-*.vsix # or: positron --install-extension ...
    ```
    The extension ID is `ggsql.ggsql`. If a previous build of the same
    version is already installed you must uninstall it first
    (`code --uninstall-extension ggsql.ggsql`) because VS Code will not
    replace an extension in place at the same version. After
    (re)installing, reload the editor window
    (**Developer: Reload Window** from the command palette) so the new
    TextMate grammar is picked up.
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
`ggsql.ggsql` extension — accept it (or install the `.vsix`
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

If `TABULATE` / `FORMAT` / `HIGHLIGHT` are **not coloured** the same
way `VISUALISE` / `DRAW` / `SCALE` are, the extension is running an
older build that predates the TABULATE grammar. Rebuild the `.vsix`
and reinstall as described in [Prerequisites](#prerequisites), then
reload the window.

## Regenerate `tabulate.ggsql`

`tabulate.ggsql` is a mechanical concatenation of every
`scenarios/NN_<slug>.ggsql` file. To add or edit scenarios:

1.  Edit or add `ggsql-vscode/examples/scenarios/<NN>_<slug>.ggsql`.
    Use the next available number.
2.  Regenerate (from any directory):
    ```sh
    python3 ggsql-vscode/examples/build_ggsql.py
    ```
3.  Commit the new / edited `scenarios/<NN>_<slug>.ggsql` along with
    the regenerated `tabulate.ggsql`.

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
