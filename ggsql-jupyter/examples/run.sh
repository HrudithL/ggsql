#!/usr/bin/env bash
# Generate, execute, and HTML-export the TABULATE Jupyter notebook.
#
# Usage (run from the repo root):
#   ./ggsql-jupyter/examples/run.sh            # uses target/debug/ggsql-jupyter
#   ./ggsql-jupyter/examples/run.sh --release  # uses target/release/ggsql-jupyter
#
# Requirements (one-time):
#   pip install jupyter nbformat nbconvert nbclient
#   cargo build -p ggsql-jupyter [--release]
#   target/<profile>/ggsql-jupyter --install --user

set -euo pipefail

cd "$(dirname "$0")"
EXAMPLES_DIR="$PWD"
REPO_ROOT="$(cd ../.. && pwd)"
OUT_DIR="$EXAMPLES_DIR/out"

PROFILE_DIR="debug"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_DIR="release"
fi

BIN="$REPO_ROOT/target/$PROFILE_DIR/ggsql-jupyter"
if [[ ! -x "$BIN" ]]; then
  echo "ggsql-jupyter binary not found at $BIN" >&2
  echo "  cargo build -p ggsql-jupyter${PROFILE_DIR:+ --release}" >&2
  exit 1
fi

# Ensure the kernel registered with jupyter points at this build.
"$BIN" --install --user >/dev/null

mkdir -p "$OUT_DIR"

echo "regenerating notebook from $EXAMPLES_DIR ..."
python3 "$EXAMPLES_DIR/build_notebook.py"

echo "executing notebook ..."
jupyter nbconvert \
  --to notebook --execute --inplace \
  --ExecutePreprocessor.kernel_name=ggsql \
  --ExecutePreprocessor.timeout=120 \
  "$EXAMPLES_DIR/tabulate.ipynb"

echo "exporting executed notebook to HTML ..."
jupyter nbconvert \
  --to html \
  --output-dir "$OUT_DIR" --output index.html \
  "$EXAMPLES_DIR/tabulate.ipynb"

echo "normalizing gt random ids so re-runs produce a stable diff ..."
python3 - <<'PY' "$OUT_DIR/index.html"
import re, sys
path = sys.argv[1]
html = open(path).read()

# gt emits per-table random tokens: a 10-hex `id="..."` on the wrapper div
# and a `gt_table_<hex>` class suffix. Replace each unique token with a
# stable sequential name in first-appearance order so consecutive runs
# produce byte-identical output.
def stabilize(html, pattern, prefix):
    seen = {}
    def repl(m):
        tok = m.group(1)
        if tok not in seen:
            seen[tok] = f"{prefix}{len(seen) + 1:04d}"
        return m.group(0).replace(tok, seen[tok])
    return re.sub(pattern, repl, html)

html = stabilize(html, r'id="([0-9a-f]{10})"', "id-")
html = stabilize(html, r'gt_table_([0-9a-z]{10,})', "gt_table_x")
open(path, "w").write(html)
PY

echo "clearing outputs and execution metadata from source notebook (committed state) ..."
python3 - <<'PY' "$EXAMPLES_DIR/tabulate.ipynb"
import sys, nbformat
path = sys.argv[1]
nb = nbformat.read(path, as_version=4)
for c in nb.cells:
    if c.cell_type == "code":
        c.execution_count = None
        c.outputs = []
    c.metadata.pop("execution", None)
nbformat.write(nb, path)
PY

echo
echo "done. open: $OUT_DIR/index.html"
