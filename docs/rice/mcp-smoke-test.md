# MCP server smoke test (owner checklist)

Needs human clicks on native prompts, so it is not automated. Run it on the
dev build after any change to the MCP server, bridge or Connections settings.

## Setup

- Dev profile: `pnpm dev` from `desktop/` (profile `app.zephium.dev`).
- Prerequisite: Tauri's `externalBin` makes every macOS build of
  `zephium-desktop` (`cargo check`, clippy, `pnpm dev`) require
  `desktop/binaries/zephium-mcp-<triple>`. Run
  `sh scripts/rice-build-mcp-bridge.sh` once before `cargo check`/`pnpm dev`
  (it also leaves `target/release/zephium-mcp`). `pnpm dev` runs the app from
  `target/`, so the dev bridge is `target/release/zephium-mcp`; a release
  bundle ships it as `Zephium.app/Contents/MacOS/zephium-mcp`.
- Dev app data dir: `D="$HOME/Library/Application Support/app.zephium.dev"`.

## Steps

1. Settings -> Connections -> Claude: switch on "Allow Claude to use Zephium".
   Check the directory and socket:
   - `ls -ld "$D/mcp"` shows `drwx------`.
   - `ls -l "$D/mcp/mcp.sock"` shows `srw-------`.
2. Register the dev bridge:
   `claude mcp add zephium-dev -e ZEPHIUM_MCP_SOCKET="$D/mcp/mcp.sock" -- "$PWD/target/release/zephium-mcp"`
3. Run `claude -p "Use the zephium tools: list my tabs, then read the active tab and summarise it in one line."`
   Expected: Zephium shows the admission prompt naming the client. After
   "Allow once", Claude lists the tabs and summarises the page. The sidebar
   shows the "... is using Zephium" indicator.
4. After step 3 finishes, check the bridge exits when the client disconnects:
   `pgrep -fl zephium-mcp` prints nothing within a few seconds. The indicator
   disappears.
5. Ask Claude to close a tab and choose Deny. Expected: the tab stays open and
   Claude reports that the person declined.
6. Open a private window and ask Claude to list tabs, then to read a private
   tab id if you know one. Expected: no private tabs are listed; a private tab
   id answers "No such tab".
7. While a call is running, click Stop on the sidebar indicator. Expected: the
   call is cancelled, the indicator disappears and Settings shows the switch
   off (Stop turns `mcp.enabled` off).
8. Check the audit log: `tail -3 "$D/mcp/mcp-audit.log"` shows JSON lines;
   `ls -l` on it shows `-rw-------`.
9. With the setting off: the socket file is gone, and the bridge reports
   "Zephium is not running, or Claude access is off ...".
10. Remove the dev registration: `claude mcp remove zephium-dev`.
