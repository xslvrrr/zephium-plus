# Upstream touchpoints

Every upstream file the rice fork edits, and why. Keep one line per file.
Rebase conflicts appear only in these files.

| File | Why |
|---|---|
| `desktop/src/updates.rs` | `SUPPORTED = false`: the fork never self-updates |
| `crates/zephium-core/src/preferences.rs` | Rice preference keys (`ui.toolbar`, `ui.window-controls`, `ui.radius`, `ui.texture`, `ui.texture-opacity`, `mcp.enabled`, `mcp.allowed`); `MAX_MCP_CLIENTS` and `mcp_allowed` for `mcp.allowed` |
| `frame/src/domain/preferences/preferences.svelte.ts` | Frame defaults for the rice preference keys (including `mcp.enabled`, `mcp.allowed`) |
| `frame/src/styles/global.css` | One `@import` of `axes/rice.css` after the last axis import |
| `frame/src/domain/appearance/theme.ts` | `applyRice`: `data-toolbar`, `--rice-radius` (the texture is drawn natively, `desktop/src/platform/macos/rice_backdrop.rs`) |
| `frame/src/app/browser/BrowserApp.svelte` | One `$effect` calling `theme.applyRice` from `ui.toolbar` and `ui.radius`; `void initMcp()` beside `focus.init()` (the MCP client list session) |
| `frame/src/app/browser/Shell.svelte` | The address block is one `addressBlock` snippet (the sidebar renders it once, as upstream's inline block); renders `<McpIndicator />` before `<SidebarNotice />` |
| `crates/zephium-core/src/layout.rs` | `Metrics.spans`: the chrome spans the window beneath the page in every shape (`sidebar()` chrome width); the page's rect is upstream's |
| `crates/zephium-core/src/windows.rs` | `Windows::iter_mut` |
| `crates/zephium-app/src/shell/mod.rs` | `chrome_spans` field, seeded from `ui.toolbar`; `Command::ChromeRegions` arm: clamps the rects to the focused window (at most 8) and calls `set_chrome_regions` |
| `crates/zephium-app/src/shell/bootstrap.rs` | Seeds `metrics.spans` on each new Main window; the chrome-reload branch clears the chrome regions (`set_chrome_regions(win.id, Vec::new())`) |
| `crates/zephium-app/src/shell/operations.rs` | `ui.toolbar` setting sets `chrome_spans`, updates all Main windows' `metrics.spans` and relayouts |
| `frame/src/features/sidebar/components/SidebarHeader.svelte` | Nav buttons extracted to `NavControls.svelte` (an extraction: upstream edits to the nav buttons conflict here and must be ported by hand into `NavControls`); `navControls` prop hides only them; the sidebar menu's opening and its `extension.action=` handling moved to `lib/sidebar-menu.ts` |
| `frame/src/features/sidebar/index.ts` | Exports `McpIndicator` |
| `desktop/src/platform/window_controls.rs` | (Task 6) `bare` ivar skips resting discs; `set_bare` |
| `desktop/src/platform/macos.rs` | (Task 6) applies `ui.window-controls` at install; `window_controls_set_bare` wrapper; `pub mod rice_backdrop;` and `rice_backdrop::install(window)` in `init` after the material |
| `desktop/src/lib.rs` | `mod rice_theme;` and `rice_theme::start(data_dir, shell)` after `external_links::adopt`; seeds `ui.radius` (`set_page_radius`) after `APP_STORE.set`; `preference.ui.window-controls=` and `preference.ui.radius=` arms in the projection hook; a macOS arm for `preference.ui.texture=` / `preference.ui.texture-opacity=` (`rice_backdrop::follow`, then `refresh_all` on the main thread), and `refresh_all` queued after `apply_native_material` in the `preference.ui.material=` arm; MCP (unix + `work-product`): `mod mcp_server;`, `app_store()` beside `app_setting`, `mcp_server::set_enabled` after `page_assist::install` when `mcp.enabled` is on, a `preference.mcp.enabled=` arm in the projection hook, `mcp_server::shutdown()` on `RunEvent::Exit`; `RegionRect` and the `chrome_regions` command (`CallerPolicy::Main`), registered in `specta_builder` after `sidebar_peek` |
| `crates/zephium-engine/src/lib.rs` | (Task 7) `PAGE_RADIUS` static with `set_page_radius` / `page_radius`; `set_chrome_regions` on `WebviewEngine` (macOS dispatch, as `raise_chrome`) |
| `crates/zephium-engine/src/host/construction.rs` | (Task 7) macOS page `configure` reads `crate::page_radius()` instead of `12.0` |
| `frame/messages/en.json` | `rice_*` strings for Settings → Appearance → Toolbar and texture; `update_status_unavailable` says the build is a fork updated by rebuilding (other locales keep upstream's text); `mcp_*` strings for Settings → Connections → Claude and the sidebar indicator |
| `frame/src/features/settings/components/sections/ConnectionsPage.svelte` | Renders `<ClaudeAccessGroup />` first, on macOS only (the MCP backend is unix-only) |
| `frame/src/features/settings/components/sections/AppearancePage.svelte` | Imports and renders `<RiceAppearance />` (toolbar, window controls, radius, texture strength, clear texture) before Typography |
| `desktop/Cargo.toml` | `notify = =8.2.0` (macos_fsevent), same pin as zephium-notes; `sha2 = =0.10.9`, the workspace pin, for the texture's media digest; `zephium-mcp-server` as an optional `cfg(unix)` dependency, enabled by `work-product`; `rfd =0.16.0` (the version and features tauri-plugin-dialog uses) also for `cfg(unix)`, so the MCP consent dialogs call rfd directly on every platform that compiles them (no lockfile change) |
| `Cargo.lock` | `zephium-desktop` gains `notify` and `sha2 0.10.9` in its dependency list (no new packages); the new `zephium-mcp-server` package (only already-locked dependencies), and `zephium-desktop` depends on it |
| `frame/src/features/newtab/lib/layout.ts` | (Task 11) `notched` option: a page without its field cuts no notch |
| `desktop/src/page_assist.rs` | Inside `mod product`: `ACTIVE` (focused space, tab, private) mirrored in `observe_items`; `PORTS` (the `DesktopPage`/`DesktopTabs` adapters, built once in `install` and shared with Ask); `mcp_ports`/`mcp_active`/`mcp_tabs`/`mcp_reader`/`mcp_outside_input` (an MCP step pauses an Ask reply acting on that tab via `human_input`) re-exported for `mcp_server`. Every `reader_switch` call now goes through `reader_needed`, so the Browse reader is on while AI or MCP needs it (`ai.enabled` in `preference_changed` and the install-time switch set the AI half) |
| `Cargo.toml` | `[workspace.dependencies]`: `zephium-mcp-server` (the rice MCP server crate) |
| `desktop/tauri.macos.conf.json` | `bundle.externalBin` ships the `zephium-mcp` bridge; macOS `beforeBuildCommand` runs `scripts/rice-build-mcp-bridge.sh` before the frame build |
| `.github/workflows/ci.yml` | macOS Rust job builds the MCP bridge first (`scripts/rice-build-mcp-bridge.sh`): tauri-build's `externalBin` copy fails without it |
| `.github/workflows/nightly.yml` | Same bridge step before the macOS coverage job's workspace build |
| `README.md` | One rice-fork note under "Build from source": run `scripts/rice-build-mcp-bridge.sh` once on macOS before `cargo check`/`pnpm dev` |
| `.cargo/config.toml` | macOS `rustflags` `-no_fixup_chains`: release builds with Xcode 27's ld otherwise write proc-macro dylibs dyld rejects |
| `vendor/wry/src/wkwebview/class/wry_web_view.rs` | `InputRegions` (a cover view and rects); `input_regions` ivar; `hitTest:` passes through over the cover outside the rects (`takes_input_at`), hit-tests as usual elsewhere; `set_input_regions` |
| `vendor/wry/src/wkwebview/mod.rs` | Initialises the `input_regions` ivar beside `skips_hit_test`; re-exports `InputRegions` (macOS) |
| `vendor/wry/src/lib.rs` | Re-exports `wkwebview::InputRegions` (macOS) beside `WryWebView` |
| `crates/zephium-core/src/ports/engine.rs` | `Engine::set_chrome_regions` (default `Unsupported`) after `raise_chrome` |
| `crates/zephium-engine/src/host/stages.rs` | macOS `set_chrome_regions` forwarding to the window's stage and keeping the list in `chrome_regions`; macOS `ensure_stage` applies a kept list to a new stage |
| `crates/zephium-engine/src/host/mod.rs` | macOS `chrome_regions: HashMap<WindowId, Vec<Rect>>` on `EngineHost` |
| `crates/zephium-engine/src/host/dispatch.rs` | Initialises `chrome_regions` (macOS) beside `stages` |
| `crates/zephium-engine/src/platform/macos/stage.rs` | `peek_raised` / `region_raised` ivars; `raise_chrome` and `set_chrome_regions` set one each and call `restack` (upstream's `raise_chrome` body, parking page tracking only for a whole-view raise); `set_chrome_regions` passes the stage as the `InputRegions` cover; `chrome_view`; `retire` clears the chrome's input regions |
| `crates/zephium-app/src/api.rs` | `Command::ChromeRegions(Vec<Rect>)` |
| `crates/zephium-app/src/actor/mailbox.rs` | `CoalescedKey::ChromeRegions`: each region list replaces the last; an empty `ChromeRegions` is critical in `command_is_critical` |
| `frame/src/shared/ipc/bindings.ts` | Generated: `commands.chromeRegions` and `RegionRect` |
