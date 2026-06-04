#!/usr/bin/env bash
set -euo pipefail

# System build deps (substrait crate needs protoc).
sudo apt-get update -qq
sudo apt-get install -y -qq protobuf-compiler

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
