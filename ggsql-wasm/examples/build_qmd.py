#!/usr/bin/env python3
"""Build ggsql-wasm/examples/tabulate.qmd from scenarios/NN_slug.ggsql.

Each .ggsql file under `scenarios/` becomes a level-2 heading + a
`{ggsql}` fenced code block that the wasm-aware Quarto extension picks
up and executes in-browser.

Run from any working directory:
    python3 ggsql-wasm/examples/build_qmd.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SCENARIOS_DIR = SCRIPT_DIR / "scenarios"
OUTPUT = SCRIPT_DIR / "tabulate.qmd"
SCENARIO_RE = re.compile(r"^\d+_.+\.ggsql$")

FRONT_MATTER = """---
title: "TABULATE examples"
subtitle: "Every NN_slug.ggsql scenario in this folder running live in your browser."
jupyter: ggsql
execute:
  enabled: true
  cache: false
format:
  html:
    toc: true
    toc-depth: 2
    page-layout: full
    include-after-body:
      - text: |
          <script type="module">
            const base = './wasm/';
            const link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = base + 'quarto.css';
            document.head.appendChild(link);
            import(base + 'quarto.js');
          </script>
---

This page is rendered by Quarto using the `ggsql` Jupyter kernel: every
`{ggsql}` fenced block below is executed at build time and the rendered
TABULATE HTML is inlined under the block. The accompanying `ggsql-wasm`
bundle (loaded via the script tag injected from the YAML header above)
then attaches a Monaco editor to each block so you can edit the query
in the browser and re-execute it client-side via WebAssembly.

The same scenarios run unchanged through the CLI
(`ggsql-cli/examples/`), the Jupyter notebook
(`ggsql-jupyter/examples/`), and the VS Code / Positron extension
(`ggsql-vscode/examples/`). The interactive playground also curates a
shorter set in its sidebar — see the **Tables** section of
[the ggsql playground][playground] or `ggsql-wasm/demo/src/examples.ts`.

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
    files = sorted(p for p in SCENARIOS_DIR.glob("*.ggsql") if SCENARIO_RE.match(p.name))
    if not files:
        print(f"no NN_*.ggsql scenario files in {SCENARIOS_DIR}", file=sys.stderr)
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
        # Negative-test scenarios should not abort the render.
        if slug.endswith("_error"):
            parts.append("#| error: true")
        parts.append(body)
        parts.append("```")

    OUTPUT.write_text("\n".join(parts).rstrip() + "\n")
    print(f"wrote {OUTPUT} ({len(files)} scenarios)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
