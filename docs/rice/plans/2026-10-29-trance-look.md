# Trance Look Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the Zephium fork Trance's silhouette (a top toolbar, hidden traffic lights, rice texture, radius and accent, and a holographic new-tab mark), driven by `rice theme apply`.

**Architecture:** Each choice is a validated preference in `zephium-core`, projected to the frame as one `data-*` axis or CSS custom property, and resolved by tokens in a fork-owned `frame/src/styles/axes/rice.css`. Native parts (layout band, window controls, page radius) read the same preferences through the shell actor and the desktop `preference.*` projection hook. A fork-owned `desktop/src/rice_theme.rs` loads `rice-theme.json` and writes preferences through the normal `Command::SetAppSetting` path.

**Tech Stack:** Rust 1.95 (Tauri 2.11, objc2), Svelte 5 runes + TypeScript, Tailwind v4 tokens, `notify` 8.2 (FSEvents), Swift `rice` CLI templates.

**Spec:** `docs/rice/2026-10-29-zephium-rice-design.md` (sections 0 and 1).

## Global Constraints

- The owner's policy: **do not run test suites**. Verify only with `cargo check`, `cargo clippy`, and `pnpm -C frame run typecheck`. Do not add `cargo test` or `vitest` steps.
- Use `python3`, never `python`. Use `pnpm` in this repo (upstream pins it). Use Bun only for scripts outside the repo.
- Do not use `!important` in fork CSS. Do not stack `backdrop-filter`. Animate only `transform` and `opacity`.
- The production CSP stays `script-src 'self'`. No `data:` or `blob:` URLs. Images come only through `mediaUrl(profile, digest, width)`.
- Presentation-barrier invariants (`docs/frontend.md`): exactly one address `HTMLInputElement`, and no duplicated `data-zephium-*` sentinels.
- Every new preference has a default that keeps stock Zephium's look.
- Every upstream file a task edits gets one line in `docs/rice/UPSTREAM-TOUCHPOINTS.md`, in the same commit.
- Commits end with:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01ML2gzDQx5BeuAPZFT135zZ
  ```
- Work on branch `rice` in `~/rice/forks/zephium`.

---

### Task 1: Fork hygiene — updater off and the touchpoints ledger

**Files:**
- Modify: `desktop/src/updates.rs:57`
- Create: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces: the ledger file that every later task appends to.

- [ ] **Step 1: Disable the updater.** Replace line 57 of `desktop/src/updates.rs`:

```rust
// rice fork: never self-update. An upstream release would replace the fork
// in /Applications; the fork is updated by rebuilding from the `rice` branch.
const SUPPORTED: bool = false;
```

`run_check` already returns the unsupported status through `updates.begin_check(SUPPORTED)`, and `schedule_native_checks` returns early.

- [ ] **Step 2: Create the ledger** at `docs/rice/UPSTREAM-TOUCHPOINTS.md`:

```markdown
# Upstream touchpoints

Every upstream file the rice fork edits, and why. Keep one line per file.
Rebase conflicts appear only in these files.

| File | Why |
|---|---|
| `desktop/src/updates.rs` | `SUPPORTED = false`: the fork never self-updates |
```

- [ ] **Step 3: Verify.**

Run: `cargo check -p zephium-desktop`
Expected: finishes with no errors. A `dead_code` warning for the now-unused update paths is acceptable; if one appears, add `#[allow(dead_code)]` only on the item it names.

- [ ] **Step 4: Fetch full history** (the clone is shallow), so later rebases work:

```bash
git fetch --unshallow upstream
```

- [ ] **Step 5: Commit**

```bash
git add desktop/src/updates.rs docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: disable self-update; add upstream touchpoints ledger"
```

---

### Task 2: The new preferences

**Files:**
- Modify: `crates/zephium-core/src/preferences.rs` (`KEYS` near line 2-68, `value_allowed` near line 197-230, the boolean bucket near line 300-320)
- Modify: `frame/src/domain/preferences/preferences.svelte.ts` (`defaults`, lines 6-68)

**Interfaces:**
- Produces these preference keys and value sets, used by every later task:
  - `ui.toolbar`: `"sidebar"` (default) or `"top"`
  - `ui.window-controls`: `"shown"` (default) or `"hover"`
  - `ui.radius`: `"0"`…`"16"`, default `"12"` (the native page radius today)
  - `ui.texture`: `""` (default) or a 64-char lowercase hex digest
  - `ui.texture-opacity`: `"0"`…`"40"`, default `"15"`
  - `ui.newtab-wordmark`: `"true"` (default), `"false"` or `"mark"`

- [ ] **Step 1: Add the keys to `KEYS`.** Append before the closing `];` of `pub const KEYS`:

```rust
    // rice fork
    "ui.toolbar",
    "ui.window-controls",
    "ui.radius",
    "ui.texture",
    "ui.texture-opacity",
```

- [ ] **Step 2: Move `ui.newtab-wordmark` out of the boolean bucket.** Delete the line `| "ui.newtab-wordmark"` from the `matches!(value, "true" | "false")` arm. Then add these arms to `value_allowed`, directly after the `"ui.newtab-image" => { ... }` arm:

```rust
        // rice fork: `mark` is Trance's centred holographic mark alone.
        "ui.newtab-wordmark" => matches!(value, "true" | "false" | "mark"),
        "ui.toolbar" => matches!(value, "sidebar" | "top"),
        "ui.window-controls" => matches!(value, "shown" | "hover"),
        "ui.radius" => value
            .parse::<u8>()
            .is_ok_and(|radius| radius <= 16 && value == radius.to_string()),
        "ui.texture" => {
            value.is_empty()
                || (value.len() == 64
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        }
        "ui.texture-opacity" => value
            .parse::<u8>()
            .is_ok_and(|opacity| opacity <= 40 && value == opacity.to_string()),
```

- [ ] **Step 3: Keep the existing unit tests compiling.** Search the file's `mod tests` for `"ui.newtab-wordmark"`. Any assertion that `value_allowed("ui.newtab-wordmark", X)` holds for X in `true`/`false` stays true. Do not add or run tests.

- [ ] **Step 4: Add the frame defaults.** In `preferences.svelte.ts`, inside `const defaults = { ... }`, before `} as const;`:

```ts
  // rice fork
  "ui.toolbar": "sidebar",
  "ui.window-controls": "shown",
  "ui.radius": "12",
  "ui.texture": "",
  "ui.texture-opacity": "15",
```

- [ ] **Step 5: Check the desktop key list.** `desktop/src/lib.rs` has `SETTING_KEYS` and `setting_value_allowed`. Confirm with `grep -n "SETTING_KEYS\s*[:=]" desktop/src/lib.rs` that it re-exports `zephium_core::preferences::KEYS` and `value_allowed`. If it is a separate literal list, append the same five keys there.

- [ ] **Step 6: Verify.**

Run: `cargo check -p zephium-core -p zephium-desktop && pnpm -C frame run typecheck`
Expected: both succeed.

- [ ] **Step 7: Commit.** Add ledger lines for `crates/zephium-core/src/preferences.rs` and `frame/src/domain/preferences/preferences.svelte.ts` (and `desktop/src/lib.rs` if Step 5 edited it), then:

```bash
git add -A crates/zephium-core/src/preferences.rs frame/src/domain/preferences/preferences.svelte.ts docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: toolbar, window-controls, radius and texture preferences"
```

---

### Task 3: The rice axis — radius, texture layer and the toolbar attribute

**Files:**
- Create: `frame/src/styles/axes/rice.css`
- Modify: `frame/src/styles/tokens.css` (the `@import` list of axes at the top; radius tokens at lines 341-358)
- Modify: `frame/src/domain/appearance/theme.ts` (add `applyRice`)
- Modify: `frame/src/app/browser/BrowserApp.svelte` (one `$effect`, next to the `theme.applyLayout` effect around line 98)
- Create: `frame/src/app/browser/RiceTexture.svelte`
- Modify: `frame/src/app/browser/Shell.svelte` (render `RiceTexture` once, next to `<SkyLight>` at line ~343)

**Interfaces:**
- Consumes: the Task 2 keys.
- Produces:
  - `theme.applyRice(axes: { toolbar: "sidebar" | "top"; radius: number; textureOpacity: number }): void`, which writes `data-toolbar` on `<html>`, plus `--rice-radius` and `--rice-texture-opacity` as inline custom properties.
  - The `<html data-toolbar="top">` attribute, read by Task 5's CSS.

- [ ] **Step 1: Find how axes are imported.** Run `grep -n "axes/" frame/src/styles/tokens.css frame/src/styles/global.css`. Add `@import "./axes/rice.css";` immediately after the last existing `axes/*.css` import, in the same file.

- [ ] **Step 2: Write `frame/src/styles/axes/rice.css`:**

```css
/* rice fork. Tokens the rice preferences resolve to. Components read the
   semantic radius tokens, never these names directly. */
:root {
  --rice-radius: 12px;
  --rice-texture-opacity: 0.15;
}

/* One radius drives the page frame and the cards around it, so the content
   view's native corner (set to the same value natively) and the chrome
   around it never disagree. */
:root {
  --content-radius: var(--rice-radius);
  --radius-card: calc(var(--rice-radius) + 6px);
  --radius-panel: calc(var(--rice-radius) + 10px);
}
```

Then, in `tokens.css`, check that `--content-radius: 12px;` (line 358) and `--radius-card` / `--radius-panel` (lines 347-348) are declared in a rule that comes *before* the `rice.css` import in the cascade. If `tokens.css` declares them after the import, move the `@import` to the end of the import block instead. Both rules use the same `:root` specificity, so the later one wins. Never use `!important`.

- [ ] **Step 3: Add `applyRice` to `theme.ts`**, directly after `applyLayout`:

```ts
/** rice fork: the toolbar's place, the shared radius and the texture's strength. */
export function applyRice(axes: {
  toolbar: "sidebar" | "top";
  radius: number;
  textureOpacity: number;
}) {
  const root = document.documentElement;
  root.setAttribute("data-toolbar", axes.toolbar);
  root.style.setProperty("--rice-radius", `${axes.radius}px`);
  root.style.setProperty("--rice-texture-opacity", String(axes.textureOpacity / 100));
}
```

- [ ] **Step 4: Call it from `BrowserApp.svelte`**, after the `theme.applyLayout` effect:

```svelte
  $effect(() => {
    const radius = Number.parseInt(preferences.value("ui.radius"), 10);
    const opacity = Number.parseInt(preferences.value("ui.texture-opacity"), 10);
    theme.applyRice({
      toolbar: preferences.value("ui.toolbar") === "top" ? "top" : "sidebar",
      radius: Number.isFinite(radius) ? radius : 12,
      textureOpacity: Number.isFinite(opacity) ? opacity : 15,
    });
  });
```

- [ ] **Step 5: Write `frame/src/app/browser/RiceTexture.svelte`.** It paints one static image, with no blur, no animation and no observers:

```svelte
<script lang="ts">
  import { mediaUrl } from "$domain/resources";
  import { preferences } from "$domain/preferences";
  import { tabs } from "$domain/tabs";

  // One bounded decode at the window's own scale; never re-requested on resize.
  const width = Math.min(1600, Math.ceil(window.screen.width * window.devicePixelRatio));
  let src = $derived.by(() => {
    const digest = preferences.value("ui.texture");
    const profile = tabs.profile()?.id;
    if (!profile || !/^[0-9a-f]{64}$/u.test(digest)) return null;
    return mediaUrl(profile, digest, width);
  });
  let failed = $state<string | null>(null);
</script>

{#if src && failed !== src}
  <img
    class="rice-texture"
    {src}
    alt=""
    aria-hidden="true"
    decoding="async"
    onerror={() => (failed = src)}
  />
{/if}

<style>
  .rice-texture {
    position: fixed;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
    object-fit: cover;
    opacity: var(--rice-texture-opacity);
    pointer-events: none;
    /* Under the chrome's own controls, over its ground. */
    z-index: 0;
    mix-blend-mode: normal;
  }

  @media (forced-colors: active) {
    .rice-texture {
      display: none;
    }
  }
</style>
```

- [ ] **Step 6: Render it once in `Shell.svelte`.** Import it with `import RiceTexture from "./RiceTexture.svelte";`. Place `<RiceTexture />` directly after the line `{#if !newTabImage()}<SkyLight shown={skyShown} />{/if}`. Check that the sidebar's and stage's own content stacks above it: with `z-index: 0` and `position: fixed`, later siblings that are positioned need `z-index` ≥ 1. If the sidebar has no positioned ancestor, add `position: relative; z-index: 1;` to `.browser-sidebar` and `.stage-column` *in `rice.css`* (scoped to `:root .browser-sidebar`), not in upstream files.

- [ ] **Step 7: Verify.**

Run: `pnpm -C frame run typecheck`
Expected: succeeds with no warnings (`--fail-on-warnings` is on).

- [ ] **Step 8: Commit.** Add ledger lines for `tokens.css` (or `global.css`), `theme.ts`, `BrowserApp.svelte` and `Shell.svelte`.

```bash
git add -A frame/src docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: radius, texture layer and toolbar axis in the frame"
```

---

### Task 4: The toolbar band in the layout core

**Files:**
- Modify: `crates/zephium-core/src/layout.rs` (`Metrics`, `Default`, `sidebar()`)
- Modify: `crates/zephium-core/src/windows.rs` (add `iter_mut`)
- Modify: `crates/zephium-app/src/shell/mod.rs` (a `toolbar_band: f64` field, initialised near line 491)
- Modify: `crates/zephium-app/src/shell/bootstrap.rs:270-272` (seed the new window)
- Modify: `crates/zephium-app/src/shell/operations.rs` (`operation_set_app_setting`, after line 896)

**Interfaces:**
- Consumes: `ui.toolbar`.
- Produces:
  - `zephium_core::layout::TOOLBAR_BAND: f64 = 44.0`, the band height in logical points, mirrored by `--rice-toolbar-height: 44px` in Task 5.
  - `Metrics.toolbar: f64` (0.0 when off).
  - `Windows::iter_mut()`.

- [ ] **Step 1: Extend `Metrics`.** In `layout.rs` add the field after `caption`:

```rust
    /// rice fork: the top toolbar band above the page. With it, the chrome
    /// spans the window so the band can run across the top of the content.
    pub toolbar: f64,
```

Add `toolbar: 0.0,` to `impl Default for Metrics`, and to the test constant `const M: Metrics` in `mod tests` (it must still compile). Add the constant near `MIN_SIDEBAR_WIDTH`:

```rust
/// rice fork: the top toolbar band's height, matching `--rice-toolbar-height`.
pub const TOOLBAR_BAND: f64 = 44.0;
```

- [ ] **Step 2: Use the band in `sidebar()`.** Replace the body from `let chrome_width = ...` to the end of the function with:

```rust
    // Hidden, the column keeps its width beneath the page, so it can come
    // over it without laying out again, and the page starts at the gutter.
    let chrome_width = if m.toolbar > 0.0 {
        // rice fork: the band runs over the content column, so the chrome
        // spans the window. The page view stacks above it inside its rect.
        win.width
    } else if m.sidebar_hidden {
        (p + m.sidebar_width + HIDDEN_SHADOW_REACH).min(win.width)
    } else {
        p + m.sidebar_width
    };
    let chrome = Rect::new(0.0, 0.0, chrome_width, win.height);
    let content_x = if m.sidebar_hidden {
        p
    } else {
        p + m.sidebar_width + m.gap
    };
    let top = m.caption + m.toolbar;
    let content = Rect::new(
        content_x,
        p + top,
        win.width - content_x - p,
        inner_h - top,
    );
    Layout {
        chrome,
        content: Some(content),
    }
```

- [ ] **Step 3: Add `iter_mut` to `Windows`** in `windows.rs`, after `iter`:

```rust
    /// Every logical window, for a change that applies to all of them.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Window> {
        self.map.values_mut()
    }
```

- [ ] **Step 4: Hold the band in the shell.** In `shell/mod.rs`, add a field beside `tab_preferences: tabs::TabPreferences,` (line ~235):

```rust
    /// rice fork: the toolbar band every window gets, from `ui.toolbar`.
    toolbar_band: f64,
```

and initialise it beside `tab_preferences: tabs::TabPreferences::load(&*store),` (line ~491):

```rust
            toolbar_band: if store.app_setting("ui.toolbar").as_deref() == Some("top") {
                zephium_core::layout::TOOLBAR_BAND
            } else {
                0.0
            },
```

- [ ] **Step 5: Seed new windows.** In `bootstrap.rs`, right after `let window = self.windows.create(WindowKind::Main, profile, space, self.pending_size);`:

```rust
        if let Some(win) = self.windows.get_mut(window) {
            win.metrics.toolbar = self.toolbar_band;
        }
```

Then run `grep -rn "windows\s*$" -A2 crates/zephium-app/src --include='*.rs' | grep "\.create("` and `grep -rn "\.create(WindowKind" crates/zephium-app/src`. Apply the same three lines after every other window creation of kind `Main`. App and Little windows keep `0.0`.

- [ ] **Step 6: React to the setting.** In `operation_set_app_setting` (`operations.rs`), after `self.tab_preferences.apply(&key, &value);`:

```rust
        if key == "ui.toolbar" {
            self.toolbar_band = if value == "top" {
                zephium_core::layout::TOOLBAR_BAND
            } else {
                0.0
            };
            for win in self.windows.iter_mut() {
                if matches!(win.kind, zephium_core::windows::WindowKind::Main) {
                    win.metrics.toolbar = self.toolbar_band;
                }
            }
            let _ = self.relayout();
        }
```

If `toolbar_band` is not visible from `operations.rs` (it is a sibling module of `mod.rs` under `shell`), a private field is still reachable because both are in the `shell` module tree. If the compiler says otherwise, mark the field `pub(super)`.

- [ ] **Step 7: Verify.**

Run: `cargo check -p zephium-core -p zephium-app -p zephium-desktop`
Expected: succeeds. Any `Metrics { ... }` literal elsewhere that fails with "missing field `toolbar`" gets `toolbar: 0.0,` added (check `crates/zephium-app/src/shell/tests/` too, so the test target still compiles under `cargo check --tests -p zephium-app`).

- [ ] **Step 8: Commit.** Ledger lines for all five files.

```bash
git add -A crates docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: toolbar band in the window layout"
```

---

### Task 5: The toolbar in the frame

**Files:**
- Create: `frame/src/features/sidebar/components/NavControls.svelte`
- Modify: `frame/src/features/sidebar/components/SidebarHeader.svelte` (replace the nav group, lines ~159-192, with `<NavControls />`)
- Modify: `frame/src/features/sidebar/index.ts` (export `NavControls`)
- Create: `frame/src/app/browser/Toolbar.svelte`
- Modify: `frame/src/app/browser/Shell.svelte` (lines ~343-390: render the address block in exactly one of the two places)
- Modify: `frame/src/features/sidebar/components/Sidebar.svelte:489-494` (`pageControls`)
- Modify: `frame/src/styles/axes/rice.css`

**Interfaces:**
- Consumes: `data-toolbar` (Task 3), `TOOLBAR_BAND = 44` (Task 4), and `preferences.value("ui.toolbar")`.
- Produces: `NavControls` (no props), and `Toolbar` with `{ children: Snippet }`.

- [ ] **Step 1: Extract `NavControls.svelte`.** Move the `{#if pageControls}<div class="flex items-center gap-0.5" role="group" ...>` block's inner `<div>…</div>` (the three `IconButton`s) out of `SidebarHeader.svelte` into a new file. Its script carries only what those buttons need:

```svelte
<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { ArrowLeft02Icon, ArrowRight02Icon, Refresh01Icon } from "@hugeicons/core-free-icons";
  import { tabs } from "$domain/tabs";
  import { commands } from "$shared/ipc/bindings";
  import IconButton from "$shared/ui/IconButton";

  let active = $derived(tabs.activeTab());
  function openHistoryMenu(forward: boolean, anchor: DOMRect) {
    const id = tabs.activeId();
    if (id) void commands.historyMenuPopup(id, forward, anchor.left, anchor.bottom);
  }
</script>

<div class="flex items-center gap-0.5" role="group" aria-label={m.ui_navigation()}>
  <IconButton
    icon={ArrowLeft02Icon}
    label={m.ui_back()}
    shape="rounded"
    buttonSize={28}
    size={16}
    disabled={active?.can_go_back !== true}
    onclick={tabs.backActive}
    onmenu={(anchor) => openHistoryMenu(false, anchor)}
  />
  <IconButton
    icon={ArrowRight02Icon}
    label={m.ui_forward()}
    shape="rounded"
    buttonSize={28}
    size={16}
    disabled={active?.can_go_forward !== true}
    onclick={tabs.forwardActive}
    onmenu={(anchor) => openHistoryMenu(true, anchor)}
  />
  <IconButton
    icon={Refresh01Icon}
    label={m.ui_reload()}
    shape="rounded"
    buttonSize={28}
    size={16}
    disabled={active === undefined}
    onclick={tabs.reloadActive}
  />
</div>
```

Before writing it, read `SidebarHeader.svelte:75-85` and copy the real body of `openHistoryMenu` if it differs from the one above. The extracted copy must behave exactly like the original. In `SidebarHeader.svelte`, replace the removed block with `{#if pageControls}<NavControls />{/if}`. Remove imports that are now unused there, because Knip and ESLint fail on them. Export it from `features/sidebar/index.ts`: `export { default as NavControls } from "./components/NavControls.svelte";`.

- [ ] **Step 2: Write `Toolbar.svelte`:**

```svelte
<script lang="ts">
  import type { Snippet } from "svelte";
  import { NavControls } from "$features/sidebar";

  let { children }: { children: Snippet } = $props();
</script>

<!-- rice fork: Trance's top band. Its height matches the native layout's
     TOOLBAR_BAND, so the page's top edge meets the band's foot. -->
<header class="rice-toolbar" data-tauri-drag-region>
  <NavControls />
  <div class="rice-toolbar-address">{@render children()}</div>
</header>

<style>
  .rice-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    block-size: var(--rice-toolbar-height);
    padding-inline: 4px 0;
    flex-shrink: 0;
  }

  .rice-toolbar-address {
    flex: 1;
    min-inline-size: 0;
  }
</style>
```

- [ ] **Step 3: Render the address block once, in one of two places.** In `Shell.svelte`, the non-compact `<AddressField {compact}>{#snippet trailing(page)}…{/snippet}</AddressField>` block (lines ~364-390) must not be duplicated. Wrap the whole block in a top-level snippet:

```svelte
{#snippet addressBlock(compact: boolean)}
  <AddressField {compact}>
    <!-- …the existing trailing snippet, moved unchanged… -->
  </AddressField>
{/snippet}
```

Add `let topToolbar = $derived(preferences.value("ui.toolbar") === "top");` to the script. In the sidebar body, replace the moved block with `{#if !topToolbar}{@render addressBlock(compact)}{/if}`. As the first child of `<div class="stage-column">`, add:

```svelte
    {#if topToolbar && !inWork}<Toolbar>{@render addressBlock(false)}</Toolbar>{/if}
```

and import it with `import Toolbar from "./Toolbar.svelte";`.

The two compact-mode `<AddressField {compact} />` renders (lines ~347 and ~353) are the rail's address glyph, not a second input. Check this by reading `AddressField.svelte` for `compact`. If compact mode renders an `<input>`, guard both with `!topToolbar` as well, so there is never a second address input.

- [ ] **Step 4: Hide the sidebar's own nav buttons in top mode.** In `Sidebar.svelte:493`, change `pageControls={!inWork}` to:

```svelte
      pageControls={!inWork && preferences.value("ui.toolbar") !== "top"}
```

and import `preferences` from `$domain/preferences` if it is not already imported.

- [ ] **Step 5: Add the band tokens to `rice.css`:**

```css
:root {
  --rice-toolbar-height: 44px;
}

/* The stage column stacks the band above the page's rect, which native
   insets by TOOLBAR_BAND. */
:root[data-toolbar="top"] .stage-column {
  display: flex;
  flex-direction: column;
}
```

Read `Shell.svelte`'s `<style>` for `.stage-column` first. If it already is a column flex container, drop that rule.

- [ ] **Step 6: Verify.**

Run: `pnpm -C frame run typecheck && pnpm -C frame run lint && pnpm -C frame run knip`
Expected: all three succeed. These are static checks, not test suites.

- [ ] **Step 7: Commit.** Ledger lines for `SidebarHeader.svelte`, `sidebar/index.ts`, `Shell.svelte` and `Sidebar.svelte`.

```bash
git add -A frame/src docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: top toolbar with navigation, address and utilities"
```

---

### Task 6: Window controls that rest invisible

**Files:**
- Modify: `desktop/src/platform/window_controls.rs` (an `Ivars` field, `draw_rect`, and a new `pub fn set_bare`)
- Modify: `desktop/src/platform/macos.rs` (call `set_bare` after `install` at line ~277)
- Modify: `desktop/src/lib.rs` (the `preference.` projection hook at line ~6974)

**Interfaces:**
- Consumes: `ui.window-controls`.
- Produces: `window_controls::set_bare(window: &WebviewWindow, bare: bool)`.

The controls already rest as unlit discs and light up under the pointer (the real AppKit buttons fade in on hover). In `hover` mode, the resting discs are simply not drawn, so the corner is empty until the pointer arrives. The hover-lit path is unchanged.

- [ ] **Step 1: Add the ivar.** In `struct Ivars`, after `suspended: Cell<bool>,`:

```rust
    /// rice fork: rest as nothing at all rather than as unlit discs.
    bare: Cell<bool>,
```

Add `bare: Cell::new(false),` wherever `Ivars { ... }` is constructed (search for `suspended: Cell::new`).

- [ ] **Step 2: Skip the resting discs.** In `draw_rect`, change the first guard to:

```rust
            if self.ivars().suspended.get() || self.ivars().bare.get() {
                return;
            }
```

- [ ] **Step 3: Add `set_bare`**, after `set_tucked`. It uses the same lookup `set_tucked` uses:

```rust
/// rice fork: whether the resting discs are drawn. Hovering still lights the
/// real buttons, so the controls stay reachable and accessible.
pub fn set_bare(window: &WebviewWindow, bare: bool) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    // SAFETY: Tao's live NSWindow, borrowed for this main-thread call only.
    let ns_window = unsafe { &*(pointer as *const NSWindow) };
    let Some(close) = ns_window.standardWindowButton(NSWindowButton::CloseButton) else {
        return;
    };
    // SAFETY: a main-thread read of the live close button's parent.
    let Some(parent) = (unsafe { close.superview() }) else {
        return;
    };
    let Some(controls) = parent
        .subviews()
        .iter()
        .find_map(|view| view.downcast_ref::<WindowControls>().map(|c| c.retain()))
    else {
        return;
    };
    controls.ivars().bare.set(bare);
    controls.setNeedsDisplay(true);
}
```

- [ ] **Step 4: Apply it at install.** In `macos.rs`, right after `super::window_controls::install(window);`:

```rust
    super::window_controls::set_bare(
        window,
        crate::app_setting("ui.window-controls").as_deref() == Some("hover"),
    );
```

- [ ] **Step 5: Follow the preference live.** In `lib.rs`'s projection hook, next to `if let Some(value) = id.strip_prefix("preference.ui.material=") { ... }`:

```rust
                    #[cfg(target_os = "macos")]
                    if let Some(value) = id.strip_prefix("preference.ui.window-controls=") {
                        let bare = value == "hover";
                        if let Some(window) = emit_handle.get_webview_window(MAIN_LABEL) {
                            let target = window.clone();
                            let _ = window.run_on_main_thread(move || {
                                platform::imp::window_controls_set_bare(&target, bare);
                            });
                        }
                    }
```

and expose it from `desktop/src/platform/macos.rs`:

```rust
pub fn window_controls_set_bare(window: &WebviewWindow, bare: bool) {
    super::window_controls::set_bare(window, bare);
}
```

Before adding the wrapper, check `grep -n "pub use\|pub mod window_controls\|mod window_controls" desktop/src/platform/*.rs`. If `window_controls` is already reachable as `platform::window_controls`, call that directly and skip the wrapper.

- [ ] **Step 6: Verify.**

Run: `cargo check -p zephium-desktop && cargo clippy -p zephium-desktop --no-deps`
Expected: no errors and no new clippy warnings.

- [ ] **Step 7: Commit.** Ledger lines for the three files.

```bash
git add -A desktop/src docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: window controls can rest invisible until hovered"
```

---

### Task 7: Native page radius follows `ui.radius`

**Files:**
- Modify: `crates/zephium-engine/src/lib.rs` (a public setter)
- Modify: `crates/zephium-engine/src/host/construction.rs:2140-2142` (read the radius instead of `12.0`)
- Modify: `desktop/src/lib.rs` (seed at startup; follow in the projection hook)

**Interfaces:**
- Consumes: `ui.radius`.
- Produces: `zephium_engine::set_page_radius(radius: f64)` and `zephium_engine::page_radius() -> f64`.

Scope (from the spec, radius bullet): a change applies to pages created after it. Existing pages keep their corner until they are recreated (for example by relaunching). rice applies at launch, so the daily path is covered. Re-rounding live views is a follow-up, not part of this task.

- [ ] **Step 1: Add the setting to the engine.** In `crates/zephium-engine/src/lib.rs`, at module level:

```rust
/// rice fork: the Browse page corner radius in points, stored as f64 bits.
static PAGE_RADIUS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0x4028_0000_0000_0000); // 12.0

/// rice fork: the corner radius new Browse pages are given.
pub fn set_page_radius(radius: f64) {
    if radius.is_finite() && (0.0..=16.0).contains(&radius) {
        PAGE_RADIUS.store(radius.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }
}

/// rice fork: the radius `set_page_radius` last stored.
pub fn page_radius() -> f64 {
    f64::from_bits(PAGE_RADIUS.load(std::sync::atomic::Ordering::Relaxed))
}
```

(`12.0_f64.to_bits()` is `0x4028_0000_0000_0000`.)

- [ ] **Step 2: Use it at construction.** In `construction.rs`, in the `#[cfg(target_os = "macos")]` call at line ~2140, replace the literal `12.0,` with `crate::page_radius(),`. Leave the other `configure` calls (`0.0` for non-page views) unchanged.

- [ ] **Step 3: Seed at startup and follow changes.** In `desktop/src/lib.rs`, find where `APP_STORE` is set (`APP_STORE.set(` or `APP_STORE.get_or_init`). Immediately after it is set:

```rust
    if let Some(radius) = app_setting("ui.radius").and_then(|value| value.parse::<f64>().ok()) {
        zephium_engine::set_page_radius(radius);
    }
```

In the projection hook, beside the Task 6 addition:

```rust
                    if let Some(value) = id.strip_prefix("preference.ui.radius=") {
                        if let Ok(radius) = value.parse::<f64>() {
                            zephium_engine::set_page_radius(radius);
                        }
                    }
```

- [ ] **Step 4: Verify.**

Run: `cargo check -p zephium-engine -p zephium-desktop`
Expected: succeeds.

- [ ] **Step 5: Commit.** Ledger lines for the three files.

```bash
git add -A crates/zephium-engine/src desktop/src/lib.rs docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: native page corner follows ui.radius"
```

---

### Task 8: The holographic new-tab mark

**Files:**
- Create: `frame/src/shared/brand/wave-light.png` (copied from Trance)
- Create: `frame/src/features/newtab/components/RiceMark.svelte`
- Modify: `frame/src/features/newtab/components/NewTab.svelte` (lines ~83, ~187, ~368, and the clock, greeting and foot sections)
- Modify: `frame/src/features/settings/components/sections/NewTabPage.svelte:119-127` (a three-way choice)

**Interfaces:**
- Consumes: `ui.newtab-wordmark = "mark"`, `ui.reduce-motion`.
- Produces: `RiceMark` (no props).

- [ ] **Step 1: Bring the art and read the source effect.**

```bash
cp ~/Downloads/Code/Trance-dev/src/zen/trance/images/wave-light.png frame/src/shared/brand/wave-light.png
sed -n 280,560p ~/Downloads/Code/Trance-dev/src/zen/trance/styles/trance-surfaces.css
sed -n 390,430p ~/Downloads/Code/Trance-dev/src/zen/trance/styles/trance-tokens.css
```

Trance is the owner's own MPL-2.0 work, so porting it is permitted. Keep Trance's sheen ramp stops, tilt range and glow values exactly. The component below uses placeholders named `--rice-sheen` and `--rice-tilt-max`; replace their values with Trance's.

- [ ] **Step 2: Write `RiceMark.svelte`.** Pointer tilt updates one CSS custom property per animation frame, and only while the pointer moves:

```svelte
<script lang="ts">
  import wave from "$shared/brand/wave-light.png";

  let tiltX = $state(0);
  let tiltY = $state(0);
  let frame = 0;
  const reduce = () => document.documentElement.getAttribute("data-reduce-motion") === "true";

  function onpointermove(event: PointerEvent) {
    if (reduce() || frame) return;
    const { clientX, clientY } = event;
    frame = requestAnimationFrame(() => {
      frame = 0;
      tiltX = (clientX / window.innerWidth - 0.5) * 2;
      tiltY = (clientY / window.innerHeight - 0.5) * 2;
    });
  }
  function onpointerleave() {
    tiltX = 0;
    tiltY = 0;
  }
</script>

<svelte:window {onpointermove} />

<div
  class="rice-mark"
  aria-hidden="true"
  style:--tilt-x={tiltX}
  style:--tilt-y={tiltY}
  onpointerleave={onpointerleave}
>
  <div class="glow"></div>
  <div class="plate" style:--wave={`url(${wave})`}>
    <img src={wave} alt="" draggable="false" />
    <div class="sheen"></div>
  </div>
</div>

<style>
  .rice-mark {
    --rice-tilt-max: 8deg; /* replace with Trance's tilt range */
    --rice-sheen: linear-gradient(
      115deg,
      #f4c4f3 0%,
      #c4e0f9 25%,
      #d7f9c4 50%,
      #f9e2c4 75%,
      #f4c4f3 100%
    ); /* replace with Trance's oil-slick ramp */
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
    perspective: 900px;
  }

  .plate {
    position: relative;
    inline-size: min(28vmin, 280px);
    aspect-ratio: 1;
    transform: rotateY(calc(var(--tilt-x) * var(--rice-tilt-max)))
      rotateX(calc(var(--tilt-y) * var(--rice-tilt-max) * -1));
    transition: transform var(--duration-slow, 400ms) var(--ease-out, ease-out);
  }

  img {
    inline-size: 100%;
    block-size: 100%;
    object-fit: contain;
  }

  /* The sheen is the mark's own shape: masked by the art, travelling with
     the tilt so it reads as holographic rather than merely coloured. */
  .sheen {
    position: absolute;
    inset: 0;
    background: var(--rice-sheen);
    background-size: 300% 300%;
    mix-blend-mode: color;
    mask: var(--wave) center / contain no-repeat;
    transform: translateX(calc(var(--tilt-x) * 6%));
    opacity: 0.85;
  }

  .glow {
    position: absolute;
    inline-size: min(34vmin, 340px);
    aspect-ratio: 1;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgb(255 255 255 / 0.12), transparent);
    opacity: 0.9;
  }

  :global([data-reduce-motion="true"]) .plate,
  :global([data-reduce-motion="true"]) .sheen {
    transform: none;
    transition: none;
  }
</style>
```

The `mask: url(...)` reads the bundled asset served from `'self'`, so the CSP holds. If Vite inlines the PNG as a `data:` URL (assets under `build.assetsInlineLimit`), the production CSP would block it. Check with `grep -n "assetsInlineLimit" frame/vite*.ts`. If it is not `0`, import the file with `?url&no-inline` instead: `import wave from "$shared/brand/wave-light.png?no-inline";`.

- [ ] **Step 3: Wire mark mode into `NewTab.svelte`.**
  - Change line 83 to `let showWordmark = $derived(preferences.value("ui.newtab-wordmark") === "true");`
  - Add `let markOnly = $derived(preferences.value("ui.newtab-wordmark") === "mark");`
  - Import it: `import RiceMark from "./RiceMark.svelte";`
  - After the `{#if showWordmark}<svg class="mark" …>…{/if}` block (line ~368), add `{#if markOnly}<RiceMark />{/if}`.
  - Wrap the greeting (`{#if showGreeting}` ~414), the clock (`{#if page.when !== null}` ~420) and the figures foot (the block containing `figure === "trackers"` ~459) in `{#if !markOnly}…{/if}`. Keep the account and customize controls (~433-445), so the page stays customizable.
  - In `layout(width, height, { … })` (~184), pass `clock: showClock && !markOnly`.

- [ ] **Step 4: Offer the choice in Settings.** In `NewTabPage.svelte`, the `ntp.wordmark` row is a switch bound to `!== "false"`. Read lines 100-140 and the existing select-style rows in the same file (for example the background `sky`/`image` choice near line 61). Replace the switch with the same kind of control, with three options: `true` → the existing `m.ntp_wordmark()` label, `mark` → a new message `ntp_wordmark_mark` ("Mark only"), and `false` → a new message `ntp_wordmark_none` ("None"). Add both messages to the English source catalog. Find it with `grep -rln "\"ntp_wordmark\"" frame/` (inlang `messages/en.json`), and add the keys only there. Other locales fall back to English.

- [ ] **Step 5: Verify.**

Run: `pnpm -C frame run typecheck && pnpm -C frame run lint && pnpm -C frame run stylelint`
Expected: all succeed.

- [ ] **Step 6: Commit.** Ledger lines for `NewTab.svelte`, `NewTabPage.svelte` and the message catalog.

```bash
git add -A frame docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: holographic mark-only new tab"
```

---

### Task 9: `rice-theme.json`, loaded and watched

**Files:**
- Create: `desktop/src/rice_theme.rs`
- Modify: `desktop/src/lib.rs` (`mod rice_theme;`, and one `rice_theme::start(dir, shell)` call where other startup services start)
- Modify: `desktop/Cargo.toml` (add `notify = { version = "=8.2.0", default-features = false, features = ["macos_fsevent"] }`, the version `zephium-notes` already pins)

**Interfaces:**
- Consumes: `zephium_core::preferences::value_allowed`, `zephium_app::Handle::{dispatch, profiles, import_media}`, `Command::SetAppSetting { key, value }`, and `MediaImport` / `MediaOrigin::Imported` (as used in `desktop/src/media.rs:76-84`).
- Produces: `rice_theme::start(dir: PathBuf, shell: zephium_app::Handle)`, where `dir` is the app data dir.
- File contract, at `<app data dir>/rice-theme.json`, where the app data dir is the directory holding `meta.sqlite`. Every field is optional:

```json
{
  "accent": "#c0c0c0",
  "texture": "/Users/ryan/rice/texture/out/topo-small.png",
  "texture_opacity": 15,
  "radius": 10,
  "toolbar": "top",
  "window_controls": "hover",
  "newtab_mark": true
}
```

- [ ] **Step 1: Find the app data dir and the startup point.**

```bash
grep -n "meta.sqlite\|app_data_dir\|fn profile_root\|APP_STORE.set" desktop/src/lib.rs | head
grep -n "schedule_native_checks(" desktop/src/lib.rs
```

Use the same directory expression the store open uses. Call `rice_theme::start` on the line after `updates::schedule_native_checks(...)`, which runs once the app and the shell handle exist.

- [ ] **Step 2: Write `desktop/src/rice_theme.rs`:**

```rust
//! rice fork: `rice-theme.json` beside the store, written by `rice theme
//! apply`. Each field present is applied as a preference at launch and on
//! every change of the file; a field left out leaves its preference alone.
//! A malformed file is logged and ignored whole.
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use zephium_app::{Command, Handle};

const FILE: &str = "rice-theme.json";
const MAX_FILE_BYTES: u64 = 16 * 1024;
const MAX_TEXTURE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RiceTheme {
    accent: Option<String>,
    texture: Option<PathBuf>,
    texture_opacity: Option<u8>,
    radius: Option<u8>,
    toolbar: Option<String>,
    window_controls: Option<String>,
    newtab_mark: Option<bool>,
}

fn read(path: &Path) -> Option<RiceTheme> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        crate::write_diagnostic(format_args!("rice-theme: not a small file, ignored"));
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(theme) => Some(theme),
        Err(error) => {
            crate::write_diagnostic(format_args!("rice-theme: malformed, ignored: {error}"));
            None
        }
    }
}

/// `#rrggbb` to the custom theme the core accepts, built from the stored
/// theme so only its accent changes.
fn accent_theme(hex: &str) -> Option<(String, String)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let rgb = [channel(0)?, channel(2)?, channel(4)?];
    let theme = crate::app_setting("ui.theme")
        .unwrap_or_else(|| zephium_core::theme::DEFAULT.to_owned());
    let custom = crate::app_setting("ui.theme-custom");
    // One accent for both appearances: rice is monochrome and dark-first.
    let carried = zephium_core::theme::with_accent(&theme, custom.as_deref(), (rgb, rgb))?;
    Some((zephium_core::theme::CUSTOM.to_owned(), carried))
}

fn set(shell: &Handle, key: &str, value: String) {
    if !zephium_core::preferences::value_allowed(key, &value) {
        crate::write_diagnostic(format_args!("rice-theme: {key} refused"));
        return;
    }
    if crate::app_setting(key).as_deref() == Some(value.as_str()) {
        return;
    }
    shell.dispatch(Command::SetAppSetting {
        key: key.to_owned(),
        value,
    });
}

fn texture_digest(shell: &Handle, path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_TEXTURE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        crate::write_diagnostic(format_args!("rice-theme: texture is not a PNG"));
        return None;
    }
    let profile = shell
        .profiles()
        .recv_timeout(Duration::from_secs(8))
        .ok()
        .flatten()?
        .current?;
    let profile = zephium_core::ids::ProfileId::parse(&profile)?;
    let receiver = shell.import_media(
        profile,
        zephium_app::MediaImport {
            request_id: format!("rice-texture-{}", zephium_core::resources::ResourceId::generate()),
            name: "rice-texture.png".into(),
            origin: zephium_app::MediaOrigin::Imported,
            bytes: std::sync::Arc::new(bytes),
        },
    );
    let reply = receiver.recv_timeout(Duration::from_secs(30)).ok()?;
    match reply.response {
        zephium_app::ResourceResponse::Applied { record, .. } => match record.draft.content {
            zephium_app::ResourceContent::Media { asset } => Some(asset.digest),
            _ => None,
        },
        _ => None,
    }
}

fn apply(shell: &Handle, path: &Path) {
    let Some(theme) = read(path) else {
        return;
    };
    if let Some((name, custom)) = theme.accent.as_deref().and_then(accent_theme) {
        set(shell, "ui.theme-custom", custom);
        set(shell, "ui.theme", name);
    }
    if let Some(opacity) = theme.texture_opacity {
        set(shell, "ui.texture-opacity", opacity.to_string());
    }
    if let Some(radius) = theme.radius {
        set(shell, "ui.radius", radius.to_string());
    }
    if let Some(toolbar) = theme.toolbar {
        set(shell, "ui.toolbar", toolbar);
    }
    if let Some(controls) = theme.window_controls {
        set(shell, "ui.window-controls", controls);
    }
    if let Some(mark) = theme.newtab_mark {
        set(shell, "ui.newtab-wordmark", if mark { "mark" } else { "true" }.to_owned());
    }
    if let Some(texture) = theme.texture.as_deref() {
        // Re-admitting identical bytes yields the same digest, so an
        // unchanged texture writes nothing.
        if let Some(digest) = texture_digest(shell, texture) {
            set(shell, "ui.texture", digest);
        }
    }
}

/// Applies the file once and then on each change, off the main thread.
pub(crate) fn start(dir: PathBuf, shell: Handle) {
    std::thread::Builder::new()
        .name("rice-theme".into())
        .spawn(move || {
            let path = dir.join(FILE);
            apply(&shell, &path);
            let (sender, events) = std::sync::mpsc::channel();
            let Ok(mut watcher) = notify::recommended_watcher(sender) else {
                return;
            };
            use notify::Watcher;
            // The directory, not the file: rice replaces the file by rename.
            if watcher.watch(&dir, notify::RecursiveMode::NonRecursive).is_err() {
                return;
            }
            while let Ok(event) = events.recv() {
                let Ok(event) = event else { continue };
                if event.paths.iter().any(|p| p.file_name() == Some(FILE.as_ref())) {
                    // Let a burst of writes from one apply settle first.
                    std::thread::sleep(Duration::from_millis(150));
                    while events.try_recv().is_ok() {}
                    apply(&shell, &path);
                }
            }
        })
        .ok();
}
```

The caller passes the app data dir from Step 1.

- [ ] **Step 3: Resolve the paths the code assumes.** Each of these must match the real code. Fix the `use` paths so they compile, and do not change the behaviour:
  - `zephium_app::{Command, Handle, MediaImport, MediaOrigin, ResourceResponse, ResourceContent}`: check them against the imports at the top of `desktop/src/media.rs`, and use the same paths.
  - `zephium_core::resources::ResourceId::generate()`: also from `media.rs`.
  - `zephium_core::theme::{DEFAULT, CUSTOM, with_accent}` and the accent tuple type `(Rgb, Rgb)`, where `Rgb = [u8; 3]`: check them against `crates/zephium-core/src/preferences.rs:140-150`, which calls `with_accent(&theme, custom.as_deref(), accent)`. If `with_accent` is `pub(crate)`, make it `pub` and add a ledger line.
  - `asset.digest`: confirm the field name with `grep -n "digest" crates/zephium-ipc/src/resources.rs`.
  - `crate::write_diagnostic` and `crate::app_setting`: both exist in `desktop/src/lib.rs`.

- [ ] **Step 4: Register and start it.** In `desktop/src/lib.rs`, add `mod rice_theme;` beside the other `mod` lines. At the startup point from Step 1, add:

```rust
            rice_theme::start(app_data_dir.clone(), shell_handle.clone());
```

Use the real local names for the data dir and the shell `Handle` at that point.

- [ ] **Step 5: Verify.**

Run: `cargo check -p zephium-desktop && cargo clippy -p zephium-desktop --no-deps`
Expected: succeeds.

- [ ] **Step 6: Commit.** Ledger lines for `desktop/src/lib.rs`, `desktop/Cargo.toml`, and `zephium-core/src/theme.rs` if it was touched.

```bash
git add -A desktop Cargo.lock crates/zephium-core/src/theme.rs docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: apply rice-theme.json at launch and on change"
```

---

### Task 10: The rice side — the `[zephium]` target

This task changes the **rice repo** (`~/rice`), not the fork. Do not commit in `~/rice`: the owner commits rice changes themselves, and the working tree there carries unrelated uncommitted work.

**Files:**
- Create: `~/rice/theme/templates/zephium-theme.json.tmpl`
- Modify: `~/rice/theme/targets.toml` (a new `[zephium]` table after `[trance]`, around line 133)

**Interfaces:**
- Consumes: the rice tokens `ramp.accent`, `texture.small-file-url` (a `file://` URL), `texture.opacity`, and `radius.*`.
- Produces: `~/Library/Application Support/app.zephium/rice-theme.json`, matching Task 9's contract.

- [ ] **Step 1: Check token spellings.** The template engine and its token names live in the Swift CLI:

```bash
grep -n "texture.small-file-url\|setting(\"" ~/rice/tools/rice/Sources/rice/Targets.swift | head
grep -n "{{" ~/rice/theme/templates/trance-prefs.js.tmpl ~/rice/theme/templates/ghostty.conf.tmpl | head
```

Confirm that `{{ texture.opacity }}` renders `0.15` and that `{{ radius.lg }}` exists. If `texture.small-file-url` is a `file://` URL, the JSON needs a plain path. Check whether the CLI has a `small-file-path` token (`grep -n "smallRenderPath" ~/rice/tools/rice/Sources/rice/*.swift`). If it does not, add one beside `texture.small-file-url` in `renderTokens`:

```swift
            .setting("texture.small-file-path",
                     to: .string(Texture.smallRenderPath(root: root).path))
```

- [ ] **Step 2: Write the template.** Trance used a 10px radius, 15% texture and motion level 2; carry the same values:

```
{
  "accent": "{{ ramp.accent }}",
  "texture": "{{ texture.small-file-path }}",
  "texture_opacity": 15,
  "radius": 10,
  "toolbar": "top",
  "window_controls": "hover",
  "newtab_mark": true
}
```

- [ ] **Step 3: Add the target** to `targets.toml`, after the `[trance]` block:

```toml
[zephium]
template = "templates/zephium-theme.json.tmpl"
output   = "~/Library/Application Support/app.zephium/rice-theme.json"
requires = "/Applications/Zephium.app"
tier     = "A"
# The Zephium rice fork (forks/zephium, branch `rice`) watches this file and
# applies each field as a preference, live. rice-owned values win at every
# launch and change, like Trance's user.js; delete a field to hand that
# setting back to Zephium's own Settings.
```

- [ ] **Step 4: Verify.** If Step 1 edited Swift, run `swift build -c release --package-path ~/rice/tools/rice`. Then run:

```bash
~/rice/tools/rice/.build/release/rice theme apply --only zephium
python3 -m json.tool "$HOME/Library/Application Support/app.zephium/rice-theme.json"
```

Expected: the apply reports 1 written and 0 failed. `json.tool` prints the object with a real `#rrggbb` accent and an absolute texture path that exists (`test -f` it).

- [ ] **Step 5: Do not commit in `~/rice`.** Report the changed rice files to the owner.

---

### Task 11: Build and compare against Trance

**Files:** none changed, unless a defect is found. A fix goes back to the task that owns the file.

- [ ] **Step 1: Install frame dependencies and run the dev build.**

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm dev
```

The dev profile is `app.zephium.dev`, so the Task 9 file for it lives in `~/Library/Application Support/app.zephium.dev/`. Copy the rendered `rice-theme.json` there for this check.

- [ ] **Step 2: Screenshot and compare.** Capture the dev window with `screencapture -l <windowid>` (find the id with `osascript` or `GetWindowID`), and Trance's window the same way. Check:
  - The band holds back, forward, reload, the address and the utilities across the top. The page starts at the band's foot, and nothing overlaps.
  - The traffic lights are absent until the pointer is in the top-left corner.
  - The texture shows at about 15%. The corners match the radius in chrome and on the page.
  - The new tab shows the wallpaper plus the centred holographic mark, with no clock or foot. The tilt follows the pointer, and is off under reduced motion.
  - Setting each preference back to its default restores stock Zephium's look.

- [ ] **Step 3: Record the result** in `docs/rice/UPSTREAM-TOUCHPOINTS.md`, under a "Verified" heading, as one line with the date and the upstream commit the fork sits on (`git merge-base HEAD upstream/main`). Then commit:

```bash
git add docs/rice/UPSTREAM-TOUCHPOINTS.md
git commit -m "rice: Trance look verified against the reference"
```
