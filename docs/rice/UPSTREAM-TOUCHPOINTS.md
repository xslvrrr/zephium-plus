# Upstream touchpoints

Every upstream file the rice fork edits, and why. Keep one line per file.
Rebase conflicts appear only in these files.

| File | Why |
|---|---|
| `desktop/src/updates.rs` | `SUPPORTED = false`: the fork never self-updates |
| `crates/zephium-core/src/preferences.rs` | Rice preference keys (`ui.toolbar`, `ui.window-controls`, `ui.radius`, `ui.texture`, `ui.texture-opacity`) and `ui.newtab-wordmark` `mark` value |
| `frame/src/domain/preferences/preferences.svelte.ts` | Frame defaults for the rice preference keys |
| `frame/src/styles/global.css` | One `@import` of `axes/rice.css` after the last axis import |
| `frame/src/domain/appearance/theme.ts` | `applyRice`: `data-toolbar`, `--rice-radius`, `--rice-texture-opacity` |
| `frame/src/app/browser/BrowserApp.svelte` | One `$effect` calling `theme.applyRice` from the rice preferences |
| `frame/src/app/browser/Shell.svelte` | Imports and renders `<RiceTexture />` next to `<SkyLight>` |
| `crates/zephium-core/src/layout.rs` | `Metrics.toolbar`, `TOOLBAR_BAND`, and `sidebar()` reserving the band above the page |
| `crates/zephium-core/src/windows.rs` | `Windows::iter_mut` |
| `crates/zephium-app/src/shell/mod.rs` | `toolbar_band` field, seeded from `ui.toolbar` |
| `crates/zephium-app/src/shell/bootstrap.rs` | Seeds `metrics.toolbar` on each new Main window |
| `crates/zephium-app/src/shell/operations.rs` | `ui.toolbar` setting updates all Main windows and relayouts |
| `frame/src/features/sidebar/components/SidebarHeader.svelte` | Nav buttons extracted to `NavControls` |
| `frame/src/features/sidebar/index.ts` | Exports `NavControls` |
| `frame/src/app/browser/Shell.svelte` | (Task 5) Address block moved into `addressBlock` snippet, rendered once in the sidebar or the top `Toolbar`; compact address guarded in top mode |
| `frame/src/features/sidebar/components/Sidebar.svelte` | `pageControls` is off when `ui.toolbar = top` |
| `desktop/src/platform/window_controls.rs` | (Task 6) `bare` ivar skips resting discs; `set_bare` |
| `desktop/src/platform/macos.rs` | (Task 6) applies `ui.window-controls` at install; `window_controls_set_bare` wrapper |
| `desktop/src/lib.rs` | (Task 6) `preference.ui.window-controls=` arm in projection hook |
