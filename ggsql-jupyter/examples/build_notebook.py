#!/usr/bin/env python3
"""Build ggsql-jupyter/examples/tabulate.ipynb from scenarios/NN_slug.ggsql.

Each .ggsql file under `scenarios/` becomes a pair of cells: a markdown
cell with the file name and any leading `--` comment as the title,
followed by a code cell containing the query (with leading comments
stripped).

Run from any working directory:
    python3 ggsql-jupyter/examples/build_notebook.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import nbformat

SCRIPT_DIR = Path(__file__).resolve().parent
SCENARIOS_DIR = SCRIPT_DIR / "scenarios"
OUTPUT = SCRIPT_DIR / "tabulate.ipynb"
SCENARIO_RE = re.compile(r"^\d+_.+\.ggsql$")


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
    files = sorted(p for p in SCENARIOS_DIR.glob("*.ggsql") if SCENARIO_RE.match(p.name))
    if not files:
        print(f"no NN_*.ggsql scenario files in {SCENARIOS_DIR}", file=sys.stderr)
        return 1

    nb = nbformat.v4.new_notebook()
    nb.metadata.kernelspec = {
        "display_name": "ggsql",
        "language": "ggsql",
        "name": "ggsql",
    }
    nb.metadata.language_info = {"name": "ggsql", "file_extension": ".ggsql"}

    intro = nbformat.v4.new_markdown_cell(
        "# TABULATE examples — Jupyter\n\n"
        "Every `NN_<slug>.ggsql` scenario in "
        "[`ggsql-jupyter/examples/`](.) executed through the `ggsql` "
        "Jupyter kernel. Each scenario is a pair of cells (markdown "
        "header + ggsql query). Output is rendered HTML via the "
        "kernel's `text/html` MIME type — the same renderer the CLI "
        "uses.\n\n"
        "Run all cells with **Run → Run All Cells**, or re-generate "
        "this notebook from disk with "
        "`python3 ggsql-jupyter/examples/build_notebook.py`."
    )
    intro.id = "intro"
    nb.cells.append(intro)

    for path in files:
        text = path.read_text()
        header, body = split_header(text)
        slug = path.stem
        title_md = f"## `{slug}.ggsql`"
        if header:
            title_md += "\n\n```\n" + header + "\n```"
        md_cell = nbformat.v4.new_markdown_cell(title_md)
        md_cell.id = f"md-{slug}"
        nb.cells.append(md_cell)
        code_cell = nbformat.v4.new_code_cell(body)
        code_cell.id = f"code-{slug}"
        if slug.endswith("_error"):
            code_cell.metadata.tags = ["raises-exception"]
        nb.cells.append(code_cell)

    nbformat.write(nb, OUTPUT)
    print(f"wrote {OUTPUT} ({len(files)} scenarios, {len(nb.cells)} cells)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
