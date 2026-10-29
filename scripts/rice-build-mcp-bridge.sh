#!/bin/sh
# rice fork: builds the MCP stdio bridge where Tauri's externalBin expects it.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
triple=$(rustc -vV | sed -n 's/^host: //p')
cargo build --manifest-path "$root/Cargo.toml" -p zephium-mcp-server --bin zephium-mcp --release
mkdir -p "$root/desktop/binaries"
cp "$root/target/release/zephium-mcp" "$root/desktop/binaries/zephium-mcp-$triple"
