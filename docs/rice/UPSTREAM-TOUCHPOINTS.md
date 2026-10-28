# Upstream touchpoints

Every upstream file the rice fork edits, and why. Keep one line per file.
Rebase conflicts appear only in these files.

| File | Why |
|---|---|
| `desktop/src/updates.rs` | `SUPPORTED = false`: the fork never self-updates |
| `crates/zephium-core/src/preferences.rs` | Rice preference keys (`ui.toolbar`, `ui.window-controls`, `ui.radius`, `ui.texture`, `ui.texture-opacity`) and `ui.newtab-wordmark` `mark` value |
| `frame/src/domain/preferences/preferences.svelte.ts` | Frame defaults for the rice preference keys |
