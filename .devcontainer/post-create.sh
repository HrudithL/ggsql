#!/usr/bin/env bash
set -euo pipefail

# System build deps:
#   - protobuf-compiler: substrait crate's build script needs protoc.
#   - unixodbc + unixodbc-dev: ggsql's odbc reader links libodbc at runtime;
#     without it, src/reader/odbc unit tests panic with "ODBC is not available".
sudo apt-get update -qq
sudo apt-get install -y -qq protobuf-compiler unixodbc unixodbc-dev

# Toolchain
rustup component add rustfmt clippy
cargo install --locked tree-sitter-cli || true

# Pull fixtures from the spec mount on first start.
if [[ -d /spec/fixtures ]]; then
  mkdir -p tests/fixtures
  rsync -a --delete /spec/fixtures/ tests/fixtures/
fi

# Sanity check
cargo --version
node --version
tree-sitter --version
echo "post-create OK"
