#!/usr/bin/env python3
"""Build examples/jupyter/tabulate.ipynb from examples/tabulate/*.ggsql.

Each .ggsql file becomes a pair of cells: a markdown cell with the file name
and any leading `--` comment as the title, followed by a code cell containing
the query (with leading comments stripped). Run this from the repo root after
adding/editing scenarios in examples/tabulate/."""

from __future__ import annotations

import sys
from pathlib import Path

import nbformat

REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE_DIR = REPO_ROOT / "examples" / "tabulate"
OUTPUT = Path(__file__).with_name("tabulate.ipynb")


def split_header(text: str) -> tuple[str, str]:
    """Return (header_lines, query_body). Header = leading --comments."""
    header, body = [], []
    in_header = True
    for line in text.splitlines():
        if in_header and (line.startswith("--") or not line.strip()):
            header.append(line)
        else:
            in_header = False
            body.append(line)
    return "\n".join(header).strip(), "\n".join(body).strip()


def main() -> int:
    files = sorted(SOURCE_DIR.glob("*.ggsql"))
    if not files:
        print(f"no .ggsql files in {SOURCE_DIR}", file=sys.stderr)
        return 1

    nb = nbformat.v4.new_notebook()
    nb.metadata.kernelspec = {
        "display_name": "ggsql",
        "language": "ggsql",
        "name": "ggsql",
    }
    nb.metadata.language_info = {"name": "ggsql", "file_extension": ".ggsql"}

    nb.cells.append(
        nbformat.v4.new_markdown_cell(
            "# TABULATE examples — Jupyter\n\n"
            "Every scenario from [`examples/tabulate/`](../tabulate/) executed "
            "through the `ggsql` Jupyter kernel. Each scenario is a pair of "
            "cells (markdown header + ggsql query). Output is rendered HTML "
            "via the kernel's `text/html` MIME type — the same renderer the "
            "CLI uses.\n\n"
            "Run all cells with **Run → Run All Cells**, or re-generate this "
            "notebook from disk with `python examples/jupyter/build_notebook.py`."
        )
    )

    for path in files:
        text = path.read_text()
        header, body = split_header(text)
        slug = path.stem
        title_md = f"## `{slug}.ggsql`"
        if header:
            title_md += "\n\n```\n" + header + "\n```"
        nb.cells.append(nbformat.v4.new_markdown_cell(title_md))
        code_cell = nbformat.v4.new_code_cell(body)
        # Examples whose filename ends in `_error` are negative tests — the
        # query should fail. Mark them so nbconvert keeps going and the error
        # traceback is preserved as the cell output.
        if slug.endswith("_error"):
            code_cell.metadata.tags = ["raises-exception"]
        nb.cells.append(code_cell)

    nbformat.write(nb, OUTPUT)
    print(f"wrote {OUTPUT} ({len(files)} scenarios, {len(nb.cells)} cells)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
