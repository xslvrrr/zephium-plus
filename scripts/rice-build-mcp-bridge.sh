#!/bin/sh
# rice fork: builds the MCP stdio bridge where Tauri's externalBin expects it.
# Usage: rice-build-mcp-bridge.sh [target-triple]   (default: host triple)
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
target_dir=${CARGO_TARGET_DIR:-$root/target}
host=$(rustc -vV | sed -n 's/^host: //p')
triple=${1:-$host}
if [ $# -ge 1 ]; then
  cargo build --manifest-path "$root/Cargo.toml" -p zephium-mcp-server --bin zephium-mcp --release --target "$1"
  built="$target_dir/$1/release/zephium-mcp"
else
  cargo build --manifest-path "$root/Cargo.toml" -p zephium-mcp-server --bin zephium-mcp --release
  built="$target_dir/release/zephium-mcp"
fi
mkdir -p "$root/desktop/binaries"
cp "$built" "$root/desktop/binaries/zephium-mcp-$triple"
