#!/usr/bin/env python3
"""Build examples/wasm/tabulate.qmd from examples/tabulate/*.ggsql.

Each .ggsql file becomes a level-2 heading + a `{ggsql}` fenced code block
that the wasm-aware Quarto extension picks up and executes in-browser. Run
this from the repo root after adding or editing scenarios in
examples/tabulate/."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE_DIR = REPO_ROOT / "examples" / "tabulate"
OUTPUT = Path(__file__).with_name("tabulate.qmd")

FRONT_MATTER = """---
title: "TABULATE examples"
subtitle: "Every scenario from `examples/tabulate/` running live in your browser."
format:
  html:
    toc: true
    toc-depth: 2
    page-layout: full
---

This page is a showcase of the `ggsql-wasm` + Quarto integration: every
`{ggsql}` fenced block below is executed by the bundled WebAssembly engine
when the page loads, and the rendered TABULATE HTML is inlined under the
block. The same scenarios run unchanged through the CLI
(`examples/cli/`), the Jupyter kernel (`examples/jupyter/`), and the VS
Code / Positron extension (`examples/vscode/`).

The interactive playground curates a shorter set of these scenarios in a
sidebar — see the **Tables** section of [the ggsql playground][playground]
or `ggsql-wasm/demo/src/examples.ts` in the repo.

[playground]: https://ggsql.org/playground
"""


def section_title(_header: str, slug: str) -> str:
    return f"## `{slug}.ggsql`"


def split_header(text: str) -> tuple[str, str]:
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

    parts: list[str] = [FRONT_MATTER]
    for path in files:
        text = path.read_text()
        header, body = split_header(text)
        slug = path.stem
        parts.append("")
        parts.append(section_title(header, slug))
        if header:
            # Render leading -- comments as prose above the cell.
            prose = "\n".join(
                line.lstrip("-").lstrip() for line in header.splitlines() if line.strip()
            )
            parts.append("")
            parts.append(prose)
        parts.append("")
        parts.append("```{ggsql}")
        parts.append(body)
        parts.append("```")

    OUTPUT.write_text("\n".join(parts).rstrip() + "\n")
    print(f"wrote {OUTPUT} ({len(files)} scenarios)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
