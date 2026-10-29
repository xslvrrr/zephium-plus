# Upstream touchpoints

Every upstream file the rice fork edits, and why. Keep one line per file.
Rebase conflicts appear only in these files.

| File | Why |
|---|---|
| `desktop/src/updates.rs` | `SUPPORTED = false`: the fork never self-updates |
| `crates/zephium-core/src/preferences.rs` | Rice preference keys (`ui.toolbar`, `ui.window-controls`, `ui.radius`, `ui.texture`, `ui.texture-opacity`, `mcp.enabled`, `mcp.allowed`) and `ui.newtab-wordmark` `mark` value; `MAX_MCP_CLIENTS` and `mcp_allowed` for `mcp.allowed` |
| `frame/src/domain/preferences/preferences.svelte.ts` | Frame defaults for the rice preference keys (including `mcp.enabled`, `mcp.allowed`) |
| `frame/src/styles/global.css` | One `@import` of `axes/rice.css` after the last axis import |
| `frame/src/domain/appearance/theme.ts` | `applyRice`: `data-toolbar`, `data-texture` (set only with a valid texture digest), `--rice-radius`, `--rice-texture-opacity` |
| `frame/src/app/browser/BrowserApp.svelte` | One `$effect` calling `theme.applyRice` from the rice preferences, including whether `ui.texture` is a valid digest; `void initMcp()` beside `focus.init()` (the MCP client list session) |
| `frame/src/app/browser/Shell.svelte` | Renders `<RiceTexture />` next to `<SkyLight>` and `<RiceTexture fill="page" />` as NewTab's `texture` snippet. The address block is one `addressBlock` snippet, rendered once: in the sidebar, or in the top `Toolbar` while `$session/rice-toolbar` `toolbarBand()` holds (there its utilities are `ToolbarUtilities`, the native sidebar menu); compact address guarded while the band shows; renders `<McpIndicator />` before `<SidebarNotice />` |
| `crates/zephium-core/src/layout.rs` | `Metrics.toolbar`, `TOOLBAR_BAND` (own doc, above `MIN_SIDEBAR_WIDTH`'s), and `sidebar()` reserving the band above the page while the sidebar is shown; hidden, the shape is upstream's exactly |
| `crates/zephium-core/src/windows.rs` | `Windows::iter_mut` |
| `crates/zephium-app/src/shell/mod.rs` | `toolbar_band` field, seeded from `ui.toolbar` |
| `crates/zephium-app/src/shell/bootstrap.rs` | Seeds `metrics.toolbar` on each new Main window |
| `crates/zephium-app/src/shell/operations.rs` | `ui.toolbar` setting updates all Main windows and relayouts |
| `frame/src/features/sidebar/components/SidebarHeader.svelte` | Nav buttons extracted to `NavControls.svelte` (an extraction: upstream edits to the nav buttons conflict here and must be ported by hand into `NavControls`); `navControls` prop hides only them; the sidebar menu's opening and its `extension.action=` handling moved to `lib/sidebar-menu.ts`, shared with the band's `ToolbarUtilities` |
| `frame/src/features/sidebar/index.ts` | Exports `NavControls` and `ToolbarUtilities`; exports `McpIndicator` |
| `frame/src/features/sidebar/components/Sidebar.svelte` | `navControls={!toolbarBand()}` on `SidebarHeader`; `pageControls` stays upstream's, so the rail menu remains in top mode |
| `desktop/src/platform/window_controls.rs` | (Task 6) `bare` ivar skips resting discs; `set_bare` |
| `desktop/src/platform/macos.rs` | (Task 6) applies `ui.window-controls` at install; `window_controls_set_bare` wrapper |
| `desktop/src/lib.rs` | `mod rice_theme;` and `rice_theme::start(data_dir, shell)` after `external_links::adopt`; seeds `ui.radius` (`set_page_radius`) after `APP_STORE.set`; `preference.ui.window-controls=` and `preference.ui.radius=` arms in the projection hook; MCP (unix + `work-product`): `mod mcp_server;`, `app_store()` beside `app_setting`, `mcp_server::set_enabled` after `page_assist::install` when `mcp.enabled` is on, a `preference.mcp.enabled=` arm in the projection hook, `mcp_server::shutdown()` on `RunEvent::Exit` |
| `crates/zephium-engine/src/lib.rs` | (Task 7) `PAGE_RADIUS` static with `set_page_radius` / `page_radius` |
| `crates/zephium-engine/src/host/construction.rs` | (Task 7) macOS page `configure` reads `crate::page_radius()` instead of `12.0` |
| `frame/src/features/newtab/components/NewTab.svelte` | `mark` wordmark mode: renders `RiceMark`, hides greeting, clock and figures, and drops the notch field, account and customize controls; `texture` snippet drawn over the page's ground |
| `frame/src/features/settings/components/sections/NewTabPage.svelte` | (Task 8) wordmark Switch becomes a three-way Select |
| `frame/messages/en.json` | `ntp_wordmark_mark`, `ntp_wordmark_none`; `rice_*` strings for Settings → Appearance → Toolbar and texture; `update_status_unavailable` says the build is a fork updated by rebuilding (other locales keep upstream's text); `mcp_*` strings for Settings → Connections → Claude and the sidebar indicator |
| `frame/src/features/settings/components/sections/ConnectionsPage.svelte` | Renders `<ClaudeAccessGroup />` first, on macOS only (the MCP backend is unix-only) |
| `frame/src/features/settings/components/sections/AppearancePage.svelte` | Imports and renders `<RiceAppearance />` (toolbar, window controls, radius, texture strength, clear texture) before Typography |
| `desktop/Cargo.toml` | `notify = =8.2.0` (macos_fsevent), same pin as zephium-notes; `sha2 = =0.10.9`, the workspace pin, for the texture's media digest; `zephium-mcp-server` as an optional `cfg(unix)` dependency, enabled by `work-product` |
| `Cargo.lock` | `zephium-desktop` gains `notify` and `sha2 0.10.9` in its dependency list (no new packages); the new `zephium-mcp-server` package (only already-locked dependencies), and `zephium-desktop` depends on it |
| `frame/src/features/newtab/lib/layout.ts` | (Task 11) `notched` option: a page without its field cuts no notch |
| `desktop/src/page_assist.rs` | Inside `mod product`: `ACTIVE` (focused space, tab, private) mirrored in `observe_items`; `PORTS` (the `DesktopPage`/`DesktopTabs` adapters, built once in `install` and shared with Ask); `mcp_ports`/`mcp_active`/`mcp_tabs`/`mcp_reader`/`mcp_outside_input` (an MCP step pauses an Ask reply acting on that tab via `human_input`) re-exported for `mcp_server`. Every `reader_switch` call now goes through `reader_needed`, so the Browse reader is on while AI or MCP needs it (`ai.enabled` in `preference_changed` and the install-time switch set the AI half) |
| `Cargo.toml` | `[workspace.dependencies]`: `zephium-mcp-server` (the rice MCP server crate) |
| `desktop/tauri.macos.conf.json` | `bundle.externalBin` ships the `zephium-mcp` bridge; macOS `beforeBuildCommand` runs `scripts/rice-build-mcp-bridge.sh` before the frame build |
