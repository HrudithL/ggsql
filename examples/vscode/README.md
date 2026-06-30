# TABULATE examples — VS Code / Positron

Every scenario from [`../tabulate/`](../tabulate/) packaged for the
[`ggsql-vscode`](../../ggsql-vscode/) extension. There is one source
artifact — [`tabulate.ggsql`](tabulate.ggsql) — split into 53 cells by
the `-- %%` delimiter the extension's cell parser recognises. Each cell
has its own **▶ Run Cell** code lens.

```
examples/vscode/
├── README.md                — this file
├── tabulate.ggsql           — 53 scenarios as cells (generated)
├── tabulate.code-workspace  — VS Code multi-root workspace + ext recommendation
└── build_ggsql.py           — regenerates tabulate.ggsql from ../tabulate/
```

## Open the workspace

```sh
code examples/vscode/tabulate.code-workspace
# or in Positron:
positron examples/vscode/tabulate.code-workspace
```

The workspace recommends the `posit.ggsql-vscode` extension. Accept the
prompt or install it manually from `ggsql-vscode/` in the source tree:

```sh
cd ggsql-vscode && npm install && npm run package
# then install the resulting .vsix from the Extensions view
```

## What to try

| Demo | What to do | What you should see |
|---|---|---|
| **Cell run** | Open `tabulate.ggsql`. Click ▶ next to `-- %% 01_minimal`. | Inline output with the rendered HTML table appears beneath the cell (Positron) or in the integrated terminal panel (plain VS Code). |
| **Run all cells** | Right-click anywhere in the file → *Run All Cells*. | All 53 scenarios execute sequentially in the kernel/session; outputs render inline. |
| **Codelens** | Notice the `▶ Run Cell` lens above every `-- %% <slug>` line. | Each cell is independently runnable; no need to select text. |
| **Connections** | Use the **Connections** pane to switch from the default in-memory `duckdb://memory` to another reader. | The next ▶ Run Cell executes against the new connection without changing the file. |
| **Negative tests** | Run the `45_facet_groups_error` cell (search for `_error`). | The cell raises a parse error inline — confirms the error surface routes back through the kernel correctly. |

## Screenshots

Drop screenshots at
[`../../ggsql-vscode/resources/screenshots/`](../../ggsql-vscode/resources/)
named:

- `tabulate-cell-run.png` — code lens + inline output
- `tabulate-positron-runtime.png` — Positron picking the ggsql runtime
- `tabulate-connections.png` — Connections pane with a TABULATE query

If they exist, they'll be referenced from the extension's main README too.

## Regenerate

Add or edit scenarios under `examples/tabulate/`, then:

```sh
python examples/vscode/build_ggsql.py
```

`tabulate.ggsql` regenerates from disk; commit alongside the upstream
`.ggsql` edits.

## How it works end-to-end

The extension is a thin Positron-aware wrapper around the `ggsql-jupyter`
kernel. When a cell runs:

1. `cellParser.ts` extracts the cell text between two `-- %%` markers.
2. The text is sent to the registered ggsql language runtime, which
   forwards it to the `ggsql-jupyter` kernel as a `execute_request`.
3. The kernel calls `ggsql::tabulate::execute::execute_with_reader` and
   then `ggsql::tabulate::html::render`, exactly like the CLI does.
4. The resulting `text/html` MIME payload is published back through the
   Jupyter protocol; the extension forwards it to Positron's output
   pane (inline, not the Plots pane).

So a regression in `examples/jupyter/out/index.html` is also a regression
here. If a scenario renders correctly in the CLI and in the Jupyter
notebook, it will render correctly through the extension.
