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

echo "regenerating notebook from $REPO_ROOT/examples/tabulate ..."
python "$EXAMPLES_DIR/build_notebook.py"

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

echo "clearing outputs from source notebook (committed state) ..."
jupyter nbconvert \
  --to notebook --inplace \
  --ClearOutputPreprocessor.enabled=True \
  "$EXAMPLES_DIR/tabulate.ipynb"

echo
echo "done. open: $OUT_DIR/index.html"
