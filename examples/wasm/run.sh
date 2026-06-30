#!/usr/bin/env bash
# Regenerate examples/wasm/tabulate.qmd and a static preview of every
# TABULATE scenario rendered through the CLI (same renderer the WASM
# engine uses).
#
# Usage:
#   examples/wasm/run.sh            # uses target/debug/ggsql
#   examples/wasm/run.sh --release  # uses target/release/ggsql

set -euo pipefail

cd "$(dirname "$0")"
EXAMPLES_DIR="$PWD"
REPO_ROOT="$(cd ../.. && pwd)"
OUT_DIR="$EXAMPLES_DIR/out"
SOURCE_DIR="$REPO_ROOT/examples/tabulate"

PROFILE_DIR="debug"
CARGO_FLAGS=()
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_DIR="release"
  CARGO_FLAGS+=("--release")
fi
BIN="$REPO_ROOT/target/$PROFILE_DIR/ggsql"
if [[ ! -x "$BIN" ]]; then
  echo "building ggsql ($PROFILE_DIR) ..."
  (cd "$REPO_ROOT" && cargo build -p ggsql-cli "${CARGO_FLAGS[@]}")
fi

echo "regenerating tabulate.qmd ..."
python "$EXAMPLES_DIR/build_qmd.py"

mkdir -p "$OUT_DIR"
INDEX="$OUT_DIR/preview.html"

cat >"$INDEX" <<'HEAD'
<!doctype html>
<meta charset="utf-8">
<title>TABULATE examples — WASM (static preview)</title>
<style>
  body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
         margin: 2rem auto; max-width: 1100px; color: #222; }
  h1 { margin-top: 0; }
  .note { background: #fffae0; border: 1px solid #f0e090;
          padding: .75rem 1rem; border-radius: 6px; margin: 1rem 0; }
  section { margin: 2.5rem 0; padding: 1rem 1.25rem;
            border: 1px solid #ddd; border-radius: 8px; background: #fafafa; }
  section h2 { margin-top: 0; font-size: 1.05rem; }
  pre { background: #f0f0f0; padding: .75rem 1rem; border-radius: 6px;
        overflow-x: auto; font-size: .85rem; }
  .table-wrap { background: white; padding: .5rem;
                border: 1px solid #eee; border-radius: 6px; overflow-x: auto; }
</style>
<h1>TABULATE examples — WASM (static preview)</h1>
<p class="note">This page is a <strong>static</strong> render of every
TABULATE scenario, produced by the CLI binary which uses the same
<code>ggsql::tabulate::html::render</code> as the WASM engine. The live,
in-browser version is <a href="../tabulate.qmd"><code>tabulate.qmd</code></a>:
open it through a Quarto site that loads the <code>ggsql-wasm</code> bundle
to see each block executed live.</p>
HEAD

shopt -s nullglob
for query in "$SOURCE_DIR"/*.ggsql; do
  name=$(basename "$query" .ggsql)
  html_path="$OUT_DIR/$name.html"
  echo "  $name.ggsql -> out/$name.html"

  if [[ "$name" == *_error ]]; then
    err_msg=$("$BIN" run "$query" 2>&1 >/dev/null) || true
    table_html="<pre style=\"color:#a00;background:#fee;padding:.75rem;border-radius:6px;\">$(python3 -c 'import sys, html; sys.stdout.write(html.escape(sys.stdin.read()))' <<<"$err_msg")</pre>"
    : >"$html_path"
  else
    "$BIN" run "$query" --output "$html_path"
    table_html=$(cat "$html_path")
  fi

  q_escaped=$(python3 -c '
import sys, html
sys.stdout.write(html.escape(open(sys.argv[1]).read()))
' "$query")

  {
    printf '<section id="%s">\n' "$name"
    printf '  <h2>%s</h2>\n' "$name"
    printf '  <pre>%s</pre>\n' "$q_escaped"
    printf '  <div class="table-wrap">%s</div>\n' "$table_html"
    printf '</section>\n'
  } >>"$INDEX"
done

echo
echo "done. open: $INDEX"
echo "      and: $EXAMPLES_DIR/tabulate.qmd"
