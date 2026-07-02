#!/usr/bin/env bash
# Run every .ggsql file in this folder through the ggsql CLI and write the
# rendered HTML to ./out/. Also produces ./out/index.html listing every
# example with its source query and rendered table.
#
# Usage:
#   examples/cli/run.sh            # build (if needed) then render all
#   examples/cli/run.sh --release  # use the release binary instead

set -euo pipefail

cd "$(dirname "$0")"
EXAMPLES_DIR="$PWD"
REPO_ROOT="$(cd ../.. && pwd)"
OUT_DIR="$EXAMPLES_DIR/out"

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

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/*.html

echo "rendering examples to $OUT_DIR"

# Start index.html
INDEX="$OUT_DIR/index.html"
cat >"$INDEX" <<'HEAD'
<!doctype html>
<meta charset="utf-8">
<title>TABULATE examples</title>
<style>
  body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
         margin: 2rem auto; max-width: 1100px; color: #222; }
  h1 { margin-top: 0; }
  section { margin: 2.5rem 0; padding: 1rem 1.25rem;
            border: 1px solid #ddd; border-radius: 8px; background: #fafafa; }
  section h2 { margin-top: 0; font-size: 1.05rem; }
  pre { background: #f0f0f0; padding: .75rem 1rem; border-radius: 6px;
        overflow-x: auto; font-size: .85rem; }
  .table-wrap { background: white; padding: .5rem;
                border: 1px solid #eee; border-radius: 6px; overflow-x: auto; }
  a.permalink { font-size: .85rem; color: #555; text-decoration: none; margin-left: .5rem; }
  a.permalink:hover { text-decoration: underline; }
</style>
<h1>TABULATE examples</h1>
<p>Rendered by <code>ggsql exec</code> using the <code>duckdb://memory</code> reader.
   Each section shows the query source and the resulting HTML table.</p>
HEAD

shopt -s nullglob
for query in "$EXAMPLES_DIR"/*.ggsql; do
  name=$(basename "$query" .ggsql)
  html_path="$OUT_DIR/$name.html"
  echo "  $name.ggsql -> out/$name.html"

  # Examples whose name ends in `_error` are negative tests: they should
  # produce a parse-/execute-time error. Capture stderr so the diagnostic
  # is what we embed in the index, and don't abort the loop on failure.
  if [[ "$name" == *_error ]]; then
    err_msg=$("$BIN" run "$query" 2>&1 >/dev/null) || true
    table_html="<pre style=\"color:#a00;background:#fee;padding:.75rem;border-radius:6px;\">$(python3 -c 'import sys, html; sys.stdout.write(html.escape(sys.stdin.read()))' <<<"$err_msg")</pre>"
    : >"$html_path"  # touch an empty stand-alone file for the index link
  else
    "$BIN" run "$query" --output "$html_path"
    table_html=$(cat "$html_path")
  fi

  # Escape the query for safe embedding in <pre>.
  q_escaped=$(python3 -c '
import sys, html
sys.stdout.write(html.escape(open(sys.argv[1]).read()))
' "$query")

  {
    printf '<section id="%s">\n' "$name"
    printf '  <h2>%s <a class="permalink" href="%s.html">(standalone)</a></h2>\n' "$name" "$name"
    printf '  <pre>%s</pre>\n' "$q_escaped"
    printf '  <div class="table-wrap">%s</div>\n' "$table_html"
    printf '</section>\n'
  } >>"$INDEX"
done

echo
echo "done. open: $OUT_DIR/index.html"
