# Notch Look Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the rice top band with a Zephium-style notch toolbar that slides over web pages on hover, group the hover window controls with the hide-sidebar button and slide them, move the rice texture to a native backdrop, and remove the new-tab wallpaper and mark.

**Architecture:** In `ui.toolbar=top` the chrome webview spans the window beneath the page. To show the notch or the window-controls group over a page, the engine raises the chrome over the stage, and the vendored `WryWebView` limits input to a list of rects so everything else reaches the page. A native mouse-moved monitor turns activation and keep-up rects into enter/leave events for the frame. The texture becomes a native layer beneath both webviews, so raising the chrome never covers or loses it.

**Tech Stack:** Rust (objc2, AppKit, Core Animation, vendored wry 0.55.1), Tauri 2.11 commands with specta bindings, Svelte 5 runes, Tailwind v4 tokens.

**Spec:** `docs/rice/2026-10-29-notch-and-site-styles-design.md` (§1, §4). Earlier spec: `docs/rice/2026-10-29-zephium-rice-design.md`.

## Global Constraints

- Do not run test suites. Verification is limited to relevant typechecks and builds: `cargo check` and `cargo clippy --all-targets` for touched crates; `pnpm --dir frame run typecheck`, `lint`, `stylelint`, `format:check`, `knip`; `pnpm --dir frame run build`; `pnpm --dir frame run budgets` on frame changes; `cargo xtask check-agentic-probe-boundary`, `check-page-reader-boundary` and `check-page-assist-boundary` when native WebKit code changes. Unit tests may be written; they are not run.
- **Ruling (bindings):** new Tauri commands need `frame/src/shared/ipc/bindings.ts` regenerated. Regenerate it by running exactly `cargo test -p zephium-desktop --lib export_typescript_bindings` (the repository's code generator, not a test suite). Never hand-edit `bindings.ts`.
- Exactly one address `<input>` in the chrome document; no duplicated `data-zephium-*` sentinels (presentation-barrier invariant).
- CSP stays `script-src 'self'`; no inline scripts.
- `frame/bundle-budgets.json`: stay within the startup budget and its 16 KiB allowance; never re-record the budget.
- New native code is macOS-only behind `#[cfg(target_os = "macos")]`; Windows and Linux must still compile (`cargo check -p <crate> --target x86_64-pc-windows-msvc` for touched crates that compile there today).
- Every upstream file edited gets one row in `docs/rice/UPSTREAM-TOUCHPOINTS.md` (update the row if it exists). New rice-only files need no row.
- Comments follow the surrounding code: prose explaining why, `rice fork:` prefix on comments inside upstream files.
- Commit after each task with a message starting `rice:` and these trailers:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01ML2gzDQx5BeuAPZFT135zZ
  ```
- Never push. Never touch `~/Library/Application Support/app.zephium` except `rice-theme.json`.
- Motion: 180 ms, `cubic-bezier(0.22, 1, 0.36, 1)` (the interface's `--ease-out`). Reduce Motion (Zephium's `ui.reduce-motion` or the system's) makes every slide instant.
- Zones: notch activation 4 px strip at the window top, the notch's width; notch keep-up = notch rect grown 32 px left, right, bottom. Group activation = 4 px strip at the window top from x=0 across the group's width, plus the traffic lights' own rect; group keep-up = group rect grown 24 px.

---

### Task 1: Remove the rice mark and the mark-only new tab

**Files:**
- Delete: `frame/src/features/newtab/components/RiceMark.svelte`
- Modify: `frame/src/features/newtab/components/NewTab.svelte` (remove `RiceMark` import, `markOnly`, every `!markOnly` guard; `showWordmark` back to upstream's `!== "false"`)
- Modify: `frame/src/features/settings/components/sections/NewTabPage.svelte` (restore upstream's wordmark `Switch`: `git show upstream/main:frame/src/features/settings/components/sections/NewTabPage.svelte`)
- Modify: `crates/zephium-core/src/preferences.rs` (`ui.newtab-wordmark` allows only `true|false` again)
- Modify: `frame/src/styles/axes/rice.css` (delete `--rice-mark-sheen`, `--rice-mark-streak` and their comment)
- Modify: `frame/messages/en.json` (delete `ntp_wordmark_mark`, `ntp_wordmark_none` if no other use)
- Delete if unused: `frame/src/shared/brand/wave-light.png` (`rg -n "wave-light" frame/src` must print nothing first)
- Modify: `desktop/src/rice_theme.rs` (drop the `newtab_mark` field and its `set` call; an unknown field is already logged, so an old `rice-theme.json` with `newtab_mark` still loads)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces: `ui.newtab-wordmark` is `true|false`. Task 8 adds a groundless new tab; the `texture` snippet prop stays until Task 3 removes it.

- [ ] **Step 1:** Delete `RiceMark.svelte`. In `NewTab.svelte` remove the `RiceMark` import, the `markOnly` derived, `{#if markOnly}<RiceMark />{/if}`, and every `&& !markOnly` / `markOnly ? [] :` guard so each expression is upstream's again (compare with `git show upstream/main:frame/src/features/newtab/components/NewTab.svelte`). Keep the `texture` prop and `notched` option for now. Set:

```ts
let showWordmark = $derived(preferences.value("ui.newtab-wordmark") !== "false");
```

- [ ] **Step 2:** In `preferences.rs`, restore the `ui.newtab-wordmark` arm to `matches!(value, "true" | "false")` and remove any rice test asserting `mark`. Restore `NewTabPage.svelte` from upstream. Remove the mark tokens from `rice.css`, the unused message keys from `en.json`, and `wave-light.png` if unused.

- [ ] **Step 3:** In `rice_theme.rs` delete the `newtab_mark` field from `RiceTheme` and its application in `apply`. Unknown fields are logged as before.

- [ ] **Step 4:** Update the touchpoints rows for `NewTab.svelte`, `NewTabPage.svelte` (remove the row if the file is now identical to upstream), `preferences.rs`, `en.json`.

- [ ] **Step 5: Verify.** `cargo check -p zephium-core -p zephium-desktop`; `cargo clippy -p zephium-core -p zephium-desktop --all-targets`; `pnpm --dir frame run typecheck && pnpm --dir frame run lint && pnpm --dir frame run knip && pnpm --dir frame run build`.

- [ ] **Step 6: Commit** `rice: remove the new-tab mark and the mark-only page`.

---

### Task 2: Make the linker workaround permanent

**Files:**
- Modify: `.cargo/config.toml`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

- [ ] **Step 1:** Add `rustflags` to both existing macOS target tables, keeping their `runner` lines:

```toml
# rice fork: ld-27037 (Xcode 27) writes proc-macro dylibs that dyld rejects
# ("mis-aligned LINKEDIT string pool") when MACOSX_DEPLOYMENT_TARGET=14.0,
# which tauri build sets from minimumSystemVersion. Without chained fixups
# the dylibs load.
[target.aarch64-apple-darwin]
runner = "scripts/dev-sign-run.sh"
rustflags = ["-C", "link-arg=-Wl,-no_fixup_chains"]

[target.x86_64-apple-darwin]
runner = "scripts/dev-sign-run.sh"
rustflags = ["-C", "link-arg=-Wl,-no_fixup_chains"]
```

- [ ] **Step 2:** Add the touchpoints row: `` `.cargo/config.toml` | macOS `rustflags` `-no_fixup_chains`: release builds with Xcode 27's ld otherwise write proc-macro dylibs dyld rejects ``.

- [ ] **Step 3: Verify.** `cargo check -p zephium-core` (confirms the config parses). Then `MACOSX_DEPLOYMENT_TARGET=14.0 cargo build --release -p zerofrom-derive --target-dir "$TMPDIR/rice-ld-check"` and confirm `python3 -I -c "import ctypes,glob; [ctypes.CDLL(p) for p in glob.glob('$TMPDIR/rice-ld-check/release/deps/libzerofrom_derive-*.dylib')]; print('ok')"` prints `ok`.

- [ ] **Step 4: Commit** `rice: link macOS builds without chained fixups`.

---

### Task 3: Native texture backdrop

**Files:**
- Create: `desktop/src/platform/rice_backdrop.rs` (macOS only)
- Modify: `desktop/src/platform/macos.rs` (declare the module; install the backdrop on each Main window beside `window_controls::install`)
- Modify: `desktop/src/lib.rs` (projection hook arms `preference.ui.texture=` and `preference.ui.texture-opacity=` call `rice_backdrop::refresh`)
- Delete: `frame/src/app/browser/RiceTexture.svelte`
- Modify: `frame/src/app/browser/Shell.svelte` (remove `<RiceTexture />`, the NewTab `texture` snippet and the import)
- Modify: `frame/src/features/newtab/components/NewTab.svelte` (remove the `texture` prop and `{@render texture?.()}`)
- Modify: `frame/src/styles/axes/rice.css` (remove the `:root[data-texture] .shell > .stage-column` rule)
- Modify: `frame/src/domain/appearance/theme.ts` (`applyRice` stops setting `data-texture`; keep `--rice-texture-opacity` only if still read, else remove it)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `ui.texture` (64-hex media digest or `""`), `ui.texture-opacity` (`0..=40`, percent), the window's material view installed by `desktop/src/material.rs`.
- Produces: `pub fn install(window: &WebviewWindow)` and `pub fn refresh_all(app: &AppHandle)` in `platform::rice_backdrop`.

- [ ] **Step 1: Find the texture bytes.** The texture is media in the profile store, admitted by `zephium_store::hub::media::admit` (see `desktop/src/rice_theme.rs:125`). Find the function the media protocol handler uses to read media by digest (`rg -n "fn .*media" crates/zephium-store/src desktop/src | rg -i "read|open|path|bytes"`). Use the same read, at full resolution, for the current profile (`rice_theme::current_profile` shows how to get it). If the store needs the shell actor, read through the same path `rice_theme.rs` uses for `import_media`.

- [ ] **Step 2: Write the backdrop.** One layer-backed `NSView` per Main window, inserted as the lowest subview of the window's content view, above the material view (`material.rs` installs the `NSVisualEffectView`; insert directly above it, below every webview), autoresizing to fill.

```rust
//! rice fork: the rice texture, drawn once per window beneath the chrome and
//! the pages. The chrome is raised over pages to show the notch; a texture
//! inside the chrome would then cover the page or vanish from behind it, so
//! it lives here instead, where nothing moves it.

use objc2::rc::Retained;
use objc2::{define_class, MainThreadOnly};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSImage, NSView, NSWindow, NSWindowOrderingMode};
use objc2_foundation::{MainThreadMarker, NSData};
use objc2_quartz_core::{kCAGravityResizeAspectFill, CALayer};
use tauri::{AppHandle, Manager, WebviewWindow};

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    pub struct RiceBackdrop;

    impl RiceBackdrop {
        // The backdrop is never a target: clicks go to the views above it.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: objc2_foundation::NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }
    }
);
```

Behaviour to implement around it:
- `install(window)`: no-op off the main thread or if a `RiceBackdrop` already exists in the content view; otherwise create it with `wantsLayer = true`, a `CALayer` whose `contentsGravity` is `kCAGravityResizeAspectFill`, `masksToBounds = true`, then call `refresh(window)`.
- `refresh(window)`: read `crate::app_setting("ui.texture")` and `crate::app_setting("ui.texture-opacity")`. Empty or invalid digest, unreadable bytes, `NSWorkspace::accessibilityDisplayShouldIncreaseContrast`, or opacity `0`: set `layer.contents = nil` and hide the view. Otherwise decode once with `NSImage::initWithData`, set `layer.contents` to the image, `layer.opacity = opacity as f32 / 100.0`, show the view. Cache the last digest per window in an ivar so an unchanged digest only updates opacity.
- `refresh_all(app)`: `refresh` every Main window (iterate `app.webview_windows()` and filter as `macos.rs` does for `window_controls`).
- Decode at most 8 MiB of bytes (`MAX_TEXTURE_BYTES` in `rice_theme.rs`, the cap on the source file).

- [ ] **Step 3: Wire it.** In `macos.rs` call `rice_backdrop::install(window)` where `window_controls::install(window)` is called for Main windows. In `lib.rs`'s preference projection hook add arms for `preference.ui.texture=` and `preference.ui.texture-opacity=` that run `platform::imp::rice_backdrop::refresh_all(&app)` on the main thread (follow the `preference.ui.radius=` arm's dispatch).

- [ ] **Step 4: Remove the frame texture.** Delete `RiceTexture.svelte`, its import and both uses in `Shell.svelte`, the `texture` prop in `NewTab.svelte`, the `data-texture` stacking rule in `rice.css`, and `data-texture` in `theme.ts` (`applyRice`). Keep `ui.texture` in Settings (the "Clear texture" action still writes `""`).

- [ ] **Step 5:** Touchpoints: update rows for `Shell.svelte`, `NewTab.svelte`, `theme.ts`, `macos.rs`, `lib.rs`; `rice.css` is rice-only.

- [ ] **Step 6: Verify.** `cargo check -p zephium-desktop`, `cargo clippy -p zephium-desktop --all-targets`, `cargo check -p zephium-desktop --target x86_64-pc-windows-msvc` (if it compiled there before this task), frame `typecheck`, `lint`, `stylelint`, `knip`, `build`, `budgets`.

- [ ] **Step 7: Commit** `rice: draw the texture natively beneath the chrome and pages`.

---

### Task 4: Drop the band; the chrome spans the window in top mode

**Files:**
- Modify: `crates/zephium-core/src/layout.rs` (replace `Metrics.toolbar: f64` and `TOOLBAR_BAND` with `Metrics.spans: bool`)
- Modify: `crates/zephium-app/src/shell/mod.rs`, `bootstrap.rs`, `operations.rs` (seed and update `spans` from `ui.toolbar`)
- Delete: `frame/src/app/browser/Toolbar.svelte`, `frame/src/session/rice-toolbar.svelte.ts`
- Delete: `frame/src/features/sidebar/components/ToolbarUtilities.svelte` (and its export in `features/sidebar/index.ts`) once nothing imports it
- Modify: `frame/src/app/browser/Shell.svelte` (remove the band render and `band` parameter of `addressBlock`; Task 8 places the notch)
- Modify: `frame/src/features/sidebar/components/Sidebar.svelte` (`navControls` follows the notch session of Task 7; until then pass `true`)
- Modify: `frame/src/styles/axes/rice.css` (remove `--rice-toolbar-height`)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces: `Metrics { spans: bool, .. }`. With `spans`, `layout::sidebar` returns `chrome = Rect(0, 0, win.width, win.height)` in every shape, and `content` exactly as upstream computes it (no band). Without it, upstream's layout exactly.

- [ ] **Step 1: Layout.** In `layout.rs` remove `TOOLBAR_BAND` and its doc, rename `Metrics.toolbar` to:

```rust
    /// rice fork: the chrome spans the window beneath the page in every
    /// shape, so it can come over the page anywhere (the notch toolbar).
    /// The page keeps upstream's rect.
    pub spans: bool,
```

Default `false`. In `sidebar()`:

```rust
    // rice fork: in notch mode the chrome spans the window beneath the page;
    // the page's own rect is upstream's in every shape.
    let chrome_width = if m.spans {
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
    let top = m.caption;
```

Update the rice layout tests to: with `spans`, chrome width equals the window width for shown and hidden sidebars, and `content` equals the `spans: false` content.

- [ ] **Step 2: Shell actor.** Rename `toolbar_band` to `chrome_spans` in `shell/mod.rs`; seed `metrics.spans = chrome_spans` in `bootstrap.rs`; in `operations.rs` the `ui.toolbar` setting sets `chrome_spans = value == "top"`, updates every Main window's `metrics.spans`, and relayouts (the existing code path, with the field renamed).

- [ ] **Step 3: Frame.** Delete `Toolbar.svelte` and `rice-toolbar.svelte.ts`. In `Shell.svelte`: remove the `toolbarBand` derived, the `{#if toolbarBand}<Toolbar>…` line, the `band` parameter (the `addressBlock` snippet keeps `UtilityTray` always), and every `!toolbarBand` guard so the sidebar renders the address block as upstream does. Task 7 adds a single `notchMode()` guard. Delete `ToolbarUtilities.svelte` and its export if `rg -n ToolbarUtilities frame/src` finds no other use.

- [ ] **Step 4:** Touchpoints: update `layout.rs`, `shell/mod.rs`, `bootstrap.rs`, `operations.rs`, `Shell.svelte`, `Sidebar.svelte`, `features/sidebar/index.ts` rows.

- [ ] **Step 5: Verify.** `cargo check -p zephium-core -p zephium-app -p zephium-desktop`; clippy the same; frame `typecheck`, `lint`, `knip`, `build`, `budgets`.

- [ ] **Step 6: Commit** `rice: drop the top band; the chrome spans the window in notch mode`.

---

### Task 5: Chrome input regions

**Files:**
- Modify: `vendor/wry/src/wkwebview/class/wry_web_view.rs` (ivar `input_rects`, `hitTest:` filter, `set_input_rects`)
- Modify: `crates/zephium-core/src/ports/engine.rs` (port method `set_chrome_regions`)
- Modify: `crates/zephium-engine/src/lib.rs` and `src/host/stages.rs` (dispatch, as `raise_chrome` does)
- Modify: `crates/zephium-engine/src/platform/macos/stage.rs` (two raise flags; apply rects)
- Modify: `crates/zephium-app/src/shell/mod.rs` (`Command::ChromeRegions(Vec<Rect>)`)
- Modify: `desktop/src/lib.rs` (Tauri command `chrome_regions`, registered in `specta_builder`)
- Regenerate: `frame/src/shared/ipc/bindings.ts`
- Create: `frame/src/session/chrome-regions.svelte.ts`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces (Rust): `WryWebView::set_input_rects(&self, rects: Option<Vec<NSRect>>)`; `EnginePort::set_chrome_regions(&self, window: WindowId, regions: Vec<Rect>) -> NativeDispatch`; Tauri `chrome_regions(caller, rects: Vec<RegionRect>)` where `RegionRect { x: f64, y: f64, width: f64, height: f64 }` in window top-left logical points.
- Produces (frame): `claimRegion(owner: symbol, rect: DOMRect | null)` and `chromeRaised(): boolean` from `$session/chrome-regions.svelte`.

- [ ] **Step 1: wry.** Add the ivar next to `skips_hit_test`, initialised `Default::default()` in `wkwebview/mod.rs` where `skips_hit_test` is:

```rust
  /// rice fork: while `Some`, the view takes the mouse only inside these
  /// rects (its own, flipped coordinates); elsewhere hit-testing passes
  /// through to the views beneath it.
  #[cfg(target_os = "macos")]
  pub(crate) input_rects: std::cell::RefCell<Option<Vec<NSRect>>>,
```

Replace `hit_test`:

```rust
    #[cfg(target_os = "macos")]
    #[unsafe(method_id(hitTest:))]
    fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
      if self.ivars().skips_hit_test.get() {
        return None;
      }
      if let Some(rects) = self.ivars().input_rects.borrow().as_ref() {
        // `point` is in the superview's coordinates.
        let superview = unsafe { self.superview() };
        let local = self.convertPoint_fromView(point, superview.as_deref());
        if !rects.iter().any(|rect| NSPointInRect(local, *rect)) {
          return None;
        }
      }
      unsafe { objc2::msg_send![super(self), hitTest: point] }
    }
```

And in `impl WryWebView`:

```rust
  /// rice fork: limits mouse input to `rects` (the view's own coordinates),
  /// or gives it the whole view again with `None`.
  pub fn set_input_rects(&self, rects: Option<Vec<NSRect>>) {
    *self.ivars().input_rects.borrow_mut() = rects;
  }
```

Import `NSPointInRect` and `NSRect` from `objc2_foundation` if not already in scope. `WKWebView` is flipped, so rects in top-left logical points map directly.

- [ ] **Step 2: Engine.** Add to the engine port, next to `raise_chrome`:

```rust
    /// rice fork: raises the chrome over the stage while `regions` is
    /// non-empty and limits its input to them; empty lowers it again unless
    /// a hidden sidebar's peek still holds it.
    fn set_chrome_regions(&self, window: WindowId, regions: Vec<Rect>) -> NativeDispatch;
```

Implement it in `crates/zephium-engine/src/lib.rs` exactly like `raise_chrome` (same `self.run`, `host::try_with`, failure message `"chrome regions were not admitted by the engine host"`), with a non-macOS arm returning `NativeDispatch::Unsupported`. Any test doubles of the port return `NativeDispatch::Unsupported`. In `host/stages.rs` add `set_chrome_regions(&mut self, window, regions)` forwarding to the macOS stage.

In `macos/stage.rs`: add ivars `peek_raised: Cell<bool>` and `region_raised: Cell<bool>`. Rename the existing body of `raise_chrome(raised)` to `fn restack(&self, raised: bool)`. Then:

```rust
    pub fn raise_chrome(&self, raised: bool) {
        self.ivars().peek_raised.set(raised);
        self.restack(raised || self.ivars().region_raised.get());
    }

    /// rice fork: the notch and the window-controls group come over the page
    /// inside these rects only; the page keeps the rest of the window.
    pub fn set_chrome_regions(&self, regions: &[Rect]) {
        let raised = !regions.is_empty();
        self.ivars().region_raised.set(raised);
        if let Some(chrome) = self.chrome_view() {
            chrome.set_input_rects(raised.then(|| {
                regions
                    .iter()
                    .map(|r| NSRect::new(NSPoint::new(r.x, r.y), NSSize::new(r.width, r.height)))
                    .collect()
            }));
        }
        self.restack(raised || self.ivars().peek_raised.get());
    }
```

`chrome_view()` returns the sibling `wry::WryWebView` that `restack` already finds (factor that lookup out). In `restack`, keep page tracking parked (the cursor cover) only when the chrome is raised **without** regions; with regions the pages keep their own cursor.

- [ ] **Step 3: Shell and command.** Add `Command::ChromeRegions(Vec<Rect>)`; on dispatch, for the focused Main window call `self.engine.set_chrome_regions(win.id, regions)` with rects clamped to the window (`Rect` intersection with `(0, 0, win.size)`; drop empty ones; at most 8). In `desktop/src/lib.rs` add:

```rust
#[derive(Clone, Copy, Debug, serde::Deserialize, specta::Type)]
pub struct RegionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// rice fork: where the chrome takes the mouse while it is over the page.
#[tauri::command]
#[specta::specta]
fn chrome_regions(caller: WebviewWindow, shell: State<'_, Handle>, rects: Vec<RegionRect>) {
    if !authorize(&caller, CallerPolicy::Main, "chrome_regions") {
        return;
    }
    let regions = rects
        .into_iter()
        .filter(|r| [r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite()))
        .filter(|r| r.width > 0.0 && r.height > 0.0)
        .take(8)
        .map(|r| Rect::new(r.x, r.y, r.width, r.height))
        .collect();
    shell.dispatch(Command::ChromeRegions(regions));
}
```

Register it in `specta_builder()`'s command list beside `sidebar_peek`. Regenerate bindings with `cargo test -p zephium-desktop --lib export_typescript_bindings`.

- [ ] **Step 4: Frame session.** Create `frame/src/session/chrome-regions.svelte.ts`:

```ts
import { commands } from "$shared/ipc/bindings";

/**
 * rice fork: the rects where the chrome takes the mouse while it is over the
 * page. Each owner (the notch, the window-controls group, a peeking card)
 * claims at most one rect; the union goes to native once per frame.
 */
const claims = new Map<symbol, DOMRect>();
let raised = $state(false);
let queued = false;

export const chromeRaised = () => raised;

export function claimRegion(owner: symbol, rect: DOMRect | null) {
  if (rect === null || rect.width <= 0 || rect.height <= 0) claims.delete(owner);
  else claims.set(owner, rect);
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    const rects = [...claims.values()].map(({ x, y, width, height }) => ({ x, y, width, height }));
    raised = rects.length > 0;
    document.documentElement.toggleAttribute("data-chrome-raised", raised);
    void commands.chromeRegions(rects);
  });
}
```

Add to `rice.css`:

```css
/* While the chrome is over the page it paints nothing there but the notch
   and the controls; the body's ground would cover the page. */
:root[data-chrome-raised] body {
  background: transparent;
}
```

- [ ] **Step 5:** Touchpoints rows for `wry_web_view.rs`, `wkwebview/mod.rs`, `ports/engine.rs`, `zephium-engine/src/lib.rs`, `host/stages.rs`, `macos/stage.rs`, `shell/mod.rs`, `desktop/src/lib.rs`, `bindings.ts` (generated).

- [ ] **Step 6: Verify.** `cargo check -p wry` (in `vendor/wry`: `cargo check --manifest-path vendor/wry/Cargo.toml`), `cargo check -p zephium-core -p zephium-engine -p zephium-app -p zephium-desktop`, clippy the same, `cargo check -p zephium-engine --target x86_64-pc-windows-msvc`, `cargo xtask check-security-fork-locks`, `cargo xtask check-agentic-probe-boundary`, frame `typecheck`, `lint`, `stylelint`, `knip`.

- [ ] **Step 7: Commit** `rice: chrome input regions over the page`.

---

### Task 6: Native pointer zones

**Files:**
- Create: `desktop/src/reach_zones.rs` (pure, platform-free)
- Create: `desktop/src/platform/macos/reach_zone.rs`
- Modify: `desktop/src/platform/macos.rs` (declare module; `reach_zones_set` wrapper)
- Modify: `desktop/src/lib.rs` (`mod reach_zones;`, Tauri command `reach_zones`)
- Regenerate: `frame/src/shared/ipc/bindings.ts`
- Create: `frame/src/session/reach-zones.svelte.ts`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces (Rust): `reach_zones::Zone { name: String, activate: Vec<Rect>, keep: Rect }`, `reach_zones::Tracker::moved(&mut self, x, y) -> Vec<(String, bool)>`.
- Produces (Tauri): `reach_zones(caller, zones: Vec<ZoneSpec>)` with `ZoneSpec { name: String, activate: Vec<RegionRect>, keep: RegionRect }`; event `zephium:reach-zone` with detail `{ name: string, inside: boolean }` sent to the caller's chrome.
- Produces (frame): `setZone(name: string, spec: { activate: DOMRect[]; keep: DOMRect } | null)` and `zoneInside(name: string): boolean`.

- [ ] **Step 1: Pure tracker.** Create `desktop/src/reach_zones.rs`:

```rust
//! rice fork: named pointer zones over the window. A zone is reached inside
//! one of its activation rects and held while the pointer stays inside its
//! larger keep rect, so a small target at the edge opens something that a
//! pointer can then move around in without it closing.

use zephium_core::geometry::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub name: String,
    pub activate: Vec<Rect>,
    pub keep: Rect,
}

#[derive(Default)]
pub struct Tracker {
    zones: Vec<(Zone, bool)>,
}

fn contains(rect: &Rect, x: f64, y: f64) -> bool {
    x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
}

impl Tracker {
    /// Replaces the zones. A zone that keeps its name keeps its state, so
    /// re-measuring an open notch does not close it.
    pub fn set(&mut self, zones: Vec<Zone>) -> Vec<(String, bool)> {
        let mut left = Vec::new();
        let old = std::mem::take(&mut self.zones);
        for (zone, inside) in &old {
            if *inside && !zones.iter().any(|z| z.name == zone.name) {
                left.push((zone.name.clone(), false));
            }
        }
        self.zones = zones
            .into_iter()
            .map(|zone| {
                let inside = old.iter().any(|(z, inside)| *inside && z.name == zone.name);
                (zone, inside)
            })
            .collect();
        left
    }

    /// The zones whose state the pointer at `(x, y)` changes.
    pub fn moved(&mut self, x: f64, y: f64) -> Vec<(String, bool)> {
        let mut changed = Vec::new();
        for (zone, inside) in &mut self.zones {
            let now = if *inside {
                contains(&zone.keep, x, y)
            } else {
                zone.activate.iter().any(|rect| contains(rect, x, y))
            };
            if now != *inside {
                *inside = now;
                changed.push((zone.name.clone(), now));
            }
        }
        changed
    }

    /// The pointer left the window: every zone is left.
    pub fn exited(&mut self) -> Vec<(String, bool)> {
        self.zones
            .iter_mut()
            .filter(|(_, inside)| *inside)
            .map(|(zone, inside)| {
                *inside = false;
                (zone.name.clone(), false)
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone() -> Zone {
        Zone {
            name: "notch".into(),
            activate: vec![Rect::new(100.0, 0.0, 200.0, 4.0)],
            keep: Rect::new(68.0, 0.0, 264.0, 82.0),
        }
    }

    #[test]
    fn reached_at_the_strip_and_held_in_the_keep() {
        let mut t = Tracker::default();
        t.set(vec![zone()]);
        assert!(t.moved(150.0, 40.0).is_empty());
        assert_eq!(t.moved(150.0, 2.0), vec![("notch".into(), true)]);
        assert!(t.moved(80.0, 70.0).is_empty());
        assert_eq!(t.moved(80.0, 90.0), vec![("notch".into(), false)]);
    }

    #[test]
    fn resetting_keeps_an_open_zone_open() {
        let mut t = Tracker::default();
        t.set(vec![zone()]);
        t.moved(150.0, 2.0);
        assert!(t.set(vec![zone()]).is_empty());
        assert_eq!(t.set(vec![]), vec![("notch".into(), false)]);
    }
}
```

- [ ] **Step 2: macOS monitor.** Create `platform/macos/reach_zone.rs` modelled on `sidebar_edge.rs`: a thread-local list of `(WebviewWindow, window number, Tracker)` and one app-local `NSEvent` monitor for `NSEventMask::MouseMoved | LeftMouseDragged`, installed while any tracker is non-empty and removed when none is. Convert `event.locationInWindow()` to top-left coordinates with the content view's height (`y = content_height - location.y`). Emit each change with `crate::emit_to_privileged(app, label, "zephium:reach-zone", &json!({ "name": name, "inside": inside }))`. Also listen for `NSEventMask::MouseExited` on the window is not available app-locally, so when a moved event belongs to another window number or `locationInWindow` falls outside the content bounds, call `exited()` for that window. `pub fn set(window: &WebviewWindow, zones: Vec<Zone>)` replaces the window's zones (and emits the leaves `Tracker::set` returns).

- [ ] **Step 3: Command.** In `lib.rs`:

```rust
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
pub struct ZoneSpec {
    pub name: String,
    pub activate: Vec<RegionRect>,
    pub keep: RegionRect,
}

/// rice fork: the zones whose reach the window reports to its chrome.
#[tauri::command]
#[specta::specta]
fn reach_zones(caller: WebviewWindow, zones: Vec<ZoneSpec>) {
    if !authorize(&caller, CallerPolicy::Main, "reach_zones") {
        return;
    }
    let rect = |r: RegionRect| Rect::new(r.x, r.y, r.width.max(0.0), r.height.max(0.0));
    let zones: Vec<reach_zones::Zone> = zones
        .into_iter()
        .filter(|z| !z.name.is_empty() && z.name.len() <= 32)
        .take(4)
        .map(|z| reach_zones::Zone {
            name: z.name,
            activate: z.activate.into_iter().take(4).map(rect).collect(),
            keep: rect(z.keep),
        })
        .collect();
    #[cfg(target_os = "macos")]
    platform::imp::reach_zones_set(&caller, zones);
    #[cfg(not(target_os = "macos"))]
    let _ = zones;
}
```

`reach_zones_set` runs `reach_zone::set` on the main thread (`caller.run_on_main_thread`). Register the command; regenerate bindings.

- [ ] **Step 4: Frame session.** Create `frame/src/session/reach-zones.svelte.ts`:

```ts
import { commands } from "$shared/ipc/bindings";

type Spec = { activate: DOMRect[]; keep: DOMRect };
const specs = new Map<string, Spec>();
const inside = $state<Record<string, boolean>>({});
let queued = false;
let listening = false;

const plain = ({ x, y, width, height }: DOMRect) => ({ x, y, width, height });

/** rice fork: whether the pointer is in a named zone (native watches it, so
 *  it is known over the page as well as over the chrome). */
export const zoneInside = (name: string) => inside[name] === true;

export function setZone(name: string, spec: Spec | null) {
  if (!listening) {
    listening = true;
    window.addEventListener("zephium:reach-zone", (event) => {
      const { name: zone, inside: now } = (event as CustomEvent<{ name: string; inside: boolean }>)
        .detail;
      inside[zone] = now;
    });
  }
  if (spec === null) specs.delete(name);
  else specs.set(name, spec);
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    void commands.reachZones(
      [...specs].map(([zone, { activate, keep }]) => ({
        name: zone,
        activate: activate.map(plain),
        keep: plain(keep),
      })),
    );
  });
}
```

Check how `zephium:sidebar-edge` is delivered (`Sidebar.svelte:301` listens on `window`); deliver `zephium:reach-zone` the same way.

- [ ] **Step 5:** Touchpoints rows for `macos.rs`, `lib.rs`, `bindings.ts`.

- [ ] **Step 6: Verify.** `cargo check -p zephium-desktop`, clippy, Windows-target check of `zephium-desktop` if it compiled before, frame `typecheck`, `lint`, `knip`.

- [ ] **Step 7: Commit** `rice: native reach zones for the notch and the window controls`.

---

### Task 7: The notch component and its session

**Files:**
- Create: `frame/src/features/notch/index.ts`
- Create: `frame/src/features/notch/components/Notch.svelte`
- Create: `frame/src/features/notch/lib/shape.ts`
- Create: `frame/src/session/notch.svelte.ts`
- Modify: `frame/src/features/newtab/lib/ground.ts` only if `notchShape` needs exporting (it is imported by `NewTab.svelte` already; prefer reuse unchanged)

**Interfaces:**
- Consumes: `notchShape`, `notchWidth` from `$features/newtab/lib/ground`; `NavControls` from `$features/sidebar`; `claimRegion` (Task 5); `setZone`, `zoneInside` (Task 6); `isHidden` from `$session/sidebar-mode.svelte`; `surface.currentPage()`; `preferences.value("ui.toolbar")`, `preferences.value("ui.reduce-motion")`.
- Produces: `notchMode(): boolean` (top toolbar, not Work); `notchDown(): boolean`; `holdNotch(owner: symbol, held: boolean)`; `<Notch content={rect} newTab={boolean}>{address snippet}</Notch>`.

- [ ] **Step 1: Session.** `frame/src/session/notch.svelte.ts`:

```ts
import { SvelteSet } from "svelte/reactivity";
import { preferences } from "$domain/preferences";
import { surface } from "$domain/surface";
import { zoneInside } from "./reach-zones.svelte";

/** rice fork: the notch toolbar holds the one address field in top mode,
 *  except in Work, which keeps the sidebar's. */
export const notchMode = () =>
  preferences.value("ui.toolbar") === "top" && surface.currentPage() !== "work";

const holds = new SvelteSet<symbol>();
let pinned = $state(false);

/** Something inside the notch needs it down: focus in the field, an open menu. */
export function holdNotch(owner: symbol, held: boolean) {
  if (held) holds.add(owner);
  else holds.delete(owner);
}

/** The new tab and Zephium's own pages keep it down. */
export function pinNotch(value: boolean) {
  pinned = value;
}

export const notchDown = () => notchMode() && (pinned || holds.size > 0 || zoneInside("notch"));
```


- [ ] **Step 2: Shape.** `lib/shape.ts` returns the SVG path for a notch of `width × height` hanging from the top, with radius 18 and flare 14, using the same geometry as the new tab's ground notch. Read `ground.ts` and call `notchShape` with `{ width, height: 50, radius: 18, flare: 14 }` if its signature fits a standalone shape; otherwise write the path here: flare arcs from `(0,0)` curving down into the side, straight sides, bottom corners of radius 18, mirrored, closed along the top edge.

- [ ] **Step 3: Component.** `Notch.svelte`:
  - Props: `content: { x: number; width: number }` (the page column in window points), `newTab: boolean`, `address: Snippet`.
  - Width: `min(640, content.width - 2 * 24)`; left: `content.x + (content.width - width) / 2`; height 50; plus the flare on both sides.
  - Markup: a `<nav data-rice-notch>` absolutely positioned at `top: 0` with `transform: translateY(0)` when down and `translateY(calc(-100% - 2px))` when up, `transition: transform 180ms cubic-bezier(0.22, 1, 0.36, 1)`; `transition: none` under `:root[data-reduce-motion]` and `@media (prefers-reduced-motion: reduce)`. Inside: an `<svg>` drawing the shape filled with the chrome surface token (`var(--color-chrome)` mixed as the sidebar's own ground is; read `material.css` for the wash the sidebar uses), then a flex row: `<NavControls />`, `{@render address()}` (flex 1), nothing else (the address field's own trailing tray already holds the site menu and extensions).
  - When `!notchDown()` and not mid-transition, the element is `inert` and `aria-hidden`.
  - Regions: a `ResizeObserver` plus `transitionend` measure `nav.getBoundingClientRect()`; while down, `claimRegion(owner, rect)` with the rect grown to include any open popover inside it (query `[data-rice-notch] [popover]:popover-open, [data-rice-notch] [role="dialog"]` and union their rects); while up, `claimRegion(owner, null)`. On the new tab (`newTab`), claim nothing: there is no page beneath.
  - Zone: on layout, `setZone("notch", { activate: [new DOMRect(left, 0, width, 4)], keep: new DOMRect(left - 32, 0, width + 64, 50 + 32) })`; on destroy `setZone("notch", null)`.
  - Holds: `focusin`/`focusout` on the nav call `holdNotch(owner, nav.contains(document.activeElement))`; an open popover inside holds it too (`toggle` events).

- [ ] **Step 4: Export** `Notch` from `features/notch/index.ts`.

- [ ] **Step 5: Verify.** Frame `typecheck`, `lint`, `stylelint`, `knip` (the component is unused until Task 8; if `knip` fails on that, do Task 8 before committing and commit both together).

- [ ] **Step 6: Commit** `rice: notch toolbar component`.

---

### Task 8: Put the notch in the shell and make the new tab groundless

**Files:**
- Modify: `frame/src/app/browser/Shell.svelte`
- Modify: `frame/src/features/sidebar/components/Sidebar.svelte` (`navControls={!notchMode()}`)
- Modify: `frame/src/features/newtab/components/NewTab.svelte` (`ground` prop: `"sky" | "none"`)
- Modify: `frame/src/features/newtab/lib/layout.ts` (keep `notched`; with no ground nothing is cut)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `Notch`, `notchMode`, `notchDown`, `pinNotch` (Task 7); `addressBlock` snippet in `Shell.svelte`.

- [ ] **Step 1: One address field.** In `Shell.svelte` add `let notched = $derived(notchMode());`. Guard the sidebar's three `AddressField` / `addressBlock` renders with `!notched` (they were the band's guards before Task 4). Render the notch once, inside `.stage-column`, before the navigation error:

```svelte
{#if notched}<Notch content={pageColumn} newTab={newTabShown}
    >{#snippet address()}{@render addressBlock(false)}{/snippet}</Notch
  >{/if}
```

`pageColumn` is the content rect's `x` and `width` in window points: measure `.stage-column` with a `ResizeObserver` (`getBoundingClientRect()`), since the stage column occupies exactly the page's rect.

- [ ] **Step 2: Pin on the new tab and Zephium pages.** `$effect(() => pinNotch(notched && (newTabShown || browserPage.currentPage() !== null)))`. Over a web page the notch is down only by zone or hold.

- [ ] **Step 3: Cmd+L.** Where the shell handles the focus-address command (`rg -n "focusAddress|address.focus|browser.focusAddress" frame/src`), in notch mode call `holdNotch(focusOwner, true)` before focusing so the notch is down when the field takes focus; the notch's own `focusout` releases it.

- [ ] **Step 4: Groundless new tab.** Add a `ground?: "sky" | "none"` prop to `NewTab.svelte` (default `"sky"`). With `"none"`: do not render the sky ground, its lip, or the notch cut (`layout(..., { notched: false })`), and do not render the `search` snippet's field (the notch's address field is the search). Keep clock, greeting, figures, account and customise controls in their layout positions. In `Shell.svelte` pass `ground={notched ? "none" : "sky"}`, and render `SkyLight` only when `!notched`.

- [ ] **Step 5: Sidebar.** `navControls={!notchMode()}` in `Sidebar.svelte`.

- [ ] **Step 6:** Touchpoints rows for `Shell.svelte`, `Sidebar.svelte`, `NewTab.svelte`, `layout.ts`.

- [ ] **Step 7: Verify.** Frame `typecheck`, `lint`, `stylelint`, `format:check`, `knip`, `build`, `budgets`. Count address inputs statically: `rg -n "<AddressField" frame/src/app/browser/Shell.svelte` shows every render guarded so exactly one can be live.

- [ ] **Step 8: Commit** `rice: notch toolbar in the shell; groundless new tab`.

---

### Task 9: Slide the window-controls group

**Files:**
- Modify: `desktop/src/platform/window_controls.rs` (`set_group_down`; `bare` kept, its hover fade removed)
- Modify: `desktop/src/platform/macos.rs` (wrapper; `ui.window-controls` at install)
- Modify: `desktop/src/lib.rs` (Tauri command `window_group(down: bool)`; projection arm for `ui.window-controls`)
- Regenerate: `frame/src/shared/ipc/bindings.ts`
- Create: `frame/src/app/browser/WindowGroup.svelte`
- Modify: `frame/src/app/browser/Shell.svelte` (render `WindowGroup` when `ui.window-controls=hover`)
- Modify: `frame/src/features/sidebar/components/SidebarHeader.svelte` (hide its own toggle button when the group owns it)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces (Rust): `window_controls::set_group_down(window: &WebviewWindow, down: bool, animate: bool)`; Tauri `window_group(caller, down: bool)`.
- Produces (frame): `<WindowGroup />` (fixed top-left; holds the hide-sidebar button).

- [ ] **Step 1: Native slide.** In `window_controls.rs` keep `bare` (no resting discs in hover mode). Add:

```rust
/// How far above its place a put-away control waits: its own height and the
/// titlebar's inset, so nothing of it shows.
const GROUP_LIFT: f64 = 40.0;
const GROUP_SECONDS: f64 = 0.18;

/// rice fork: brings the controls down into the corner, or slides them up
/// out of the window. The buttons move by their layers' transform, never
/// their frames, so Tao's inset and AppKit's titlebar layout are untouched;
/// put away they are hidden as well, so they take no clicks.
pub fn set_group_down(window: &WebviewWindow, down: bool, animate: bool) {
```

Implementation: find `controls` as `set_bare` does. For each `button` in `controls.buttons()`: if `down`, `button.setHidden(false)` first. Animate `transform.translation.y` on the button's layer from its presentation value (continue mid-flight, as `fade` does with opacity) to `0` when down or `GROUP_LIFT` when up (AppKit layers are not flipped: positive y is up), duration `GROUP_SECONDS` unless `animate` is false or `accessibilityDisplayShouldReduceMotion`, timing function control points `0.22, 1.0, 0.36, 1.0`. Set the model value with `CATransaction::setDisableActions(true)`. When up, hide the buttons in the animation's completion (`CATransaction::setCompletionBlock`), checking the latest requested state so a reversal mid-flight does not hide them. Also light them (`light(true, false)`) while down so they show their colours; up, `light(false, false)`. Remove the old hover-fade path for hover mode (bare mode no longer fades on `mouse_entered`; guard those handlers with `!bare`).

- [ ] **Step 2: Command and preference.** `window_group(caller, down)` authorizes `CallerPolicy::Main` and calls `platform::imp::window_group(&caller, down)` on the main thread (which calls `set_group_down(window, down, true)`). At install and on `preference.ui.window-controls=`: `hover` sets `bare` and `set_group_down(window, false, false)`; `shown` clears `bare` and `set_group_down(window, true, false)`. Interaction with `set_tucked` (hidden sidebar): in hover mode `set_tucked` must not show the buttons; the group decides. Read `set_tucked` and gate its `setHidden(false)` on `!bare`.

Regenerate bindings.

- [ ] **Step 3: Frame group.** `WindowGroup.svelte`:
  - Fixed at `top: 0; left: 0`, height `var(--titlebar-height)`, padding-inline-start `var(--traffic-light-inset)` plus the lights' width (read how `SidebarHeader.svelte` reserves `lights` space and reuse its tokens), containing one `IconButton` with `SidebarLeftIcon`, label `m.ui_compact_mode()`, `onclick={toggleMode}` from `$session/sidebar-mode.svelte`.
  - Slides with the same transform transition as the notch (Task 7), `inert` while up.
  - Down when `zoneInside("group")` or the button has focus.
  - Zone: `setZone("group", { activate: [new DOMRect(0, 0, groupWidth, 4), lightsRect], keep: groupRect grown by 24 })`, where `lightsRect` is the traffic lights' rect (`var(--traffic-light-inset)` to the button's left edge, the titlebar's height).
  - On each change of down, call `commands.windowGroup(down)` so native slides the buttons in the same frame.
  - Region: while down and the sidebar is hidden (`isHidden()`) or the chrome is otherwise beneath a page there, `claimRegion(owner, groupRect)`; when the sidebar is shown the group is over the sidebar, which is chrome already, so claim nothing.

- [ ] **Step 4: Header.** In `SidebarHeader.svelte`, when `preferences.value("ui.window-controls") === "hover"` and not `launcher`, render the lights spacer but not the `SidebarLeftIcon` button (the group draws it in the same place). Add a `rice fork:` comment.

- [ ] **Step 5:** Render `{#if preferences.value("ui.window-controls") === "hover" && IS_MAC}<WindowGroup />{/if}` in `Shell.svelte` once, after `<Sidebar>`.

- [ ] **Step 6:** Touchpoints rows for `window_controls.rs`, `macos.rs`, `lib.rs`, `SidebarHeader.svelte`, `Shell.svelte`, `bindings.ts`.

- [ ] **Step 7: Verify.** `cargo check -p zephium-desktop`, clippy, frame `typecheck`, `lint`, `stylelint`, `knip`, `build`, `budgets`.

- [ ] **Step 8: Commit** `rice: window controls and the sidebar button slide in as one group`.

---

### Task 10: The hidden sidebar's peek in notch mode

**Files:**
- Modify: `frame/src/features/sidebar/components/Sidebar.svelte`
- Modify: `crates/zephium-app/src/shell/mod.rs` (`PeekSidebar`: in spanning mode raise only through regions)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `claimRegion` (Task 5); `layout::hidden_card`.

- [ ] **Step 1:** With `metrics.spans`, the chrome beneath the page is the whole window, so a peek that raises it whole would take every click on the page. In `shell/mod.rs` `Command::PeekSidebar`: when `win.metrics.spans`, skip `raise_chrome` (the frame claims the card as a region instead); otherwise upstream's behaviour exactly.

- [ ] **Step 2:** In `Sidebar.svelte`, while `peeking` and `notchMode()`, `claimRegion(cardOwner, cardRect)` where `cardRect` is the card element's `getBoundingClientRect()` grown by its shadow reach (`HIDDEN_SHADOW_REACH`, 64 px, on the right only); when the peek ends, `claimRegion(cardOwner, null)`. Measure on the same frames the peek's slide ends.

- [ ] **Step 3:** Touchpoints rows for `Sidebar.svelte`, `shell/mod.rs`.

- [ ] **Step 4: Verify.** `cargo check -p zephium-app`, clippy, frame `typecheck`, `lint`, `knip`.

- [ ] **Step 5: Commit** `rice: a peeking sidebar in notch mode takes only its card`.

---

### Task 11: Rice theme template, docs and the owner checklist

**Files:**
- Modify (rice repo): `~/rice/theme/templates/zephium-theme.json.tmpl` (remove `newtab_mark`)
- Modify: `docs/rice/2026-10-29-zephium-rice-design.md` (point §1's band and mark text at the new spec)
- Create: `docs/rice/notch-smoke-test.md`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md` (final pass: every file of Tasks 1-10 has its row; rows for deleted rice code are gone)

- [ ] **Step 1:** Remove the `newtab_mark` line from the template (keep the JSON valid: watch the trailing comma). Render it with `cd ~/rice && rice theme apply` only if the owner's `rice` binary is built (`test -x tools/rice/.build/release/rice`); otherwise leave rendering to the owner. Commit in `~/rice`: `Zephium template: drop newtab_mark` with the trailers. **Ruling:** the owner asked to commit everything earlier in this session, so committing this one-line template change in `~/rice` is in scope.

- [ ] **Step 2:** Add a note at the top of the earlier spec's §1: "Band, mark and mark-only new tab superseded by `2026-10-29-notch-and-site-styles-design.md` §1."

- [ ] **Step 3:** Write `docs/rice/notch-smoke-test.md` with numbered checks:
  1. New tab: notch down, address field in it, no wallpaper, texture behind the page.
  2. Web page: notch hidden; move to the top-centre edge: it slides down; move 30 px below it: it stays; move away: it slides up.
  3. With the notch down, click and scroll the page to the notch's left and right: the page responds.
  4. Cmd+L on a page: notch comes down with the field focused; Escape or clicking the page puts it away.
  5. The address field's site menu opens below the notch and stays usable.
  6. Hover mode: move to the top-left corner: traffic lights and the sidebar button slide down together; the button hides and shows the sidebar; moving away slides them up; with them up, clicking where they were reaches the page.
  7. Hidden sidebar: the left-edge peek still works and the page to its right stays clickable.
  8. Reduce Motion on: every slide is instant.
  9. Sidebar mode (`ui.toolbar=sidebar`): upstream's look, address in the sidebar.

- [ ] **Step 4: Verify.** `python3 -I -c "import json,sys; json.load(open(sys.argv[1]))" <(sed 's/{{[^}]*}}/0/g' ~/rice/theme/templates/zephium-theme.json.tmpl)` prints nothing (valid JSON shape).

- [ ] **Step 5: Commit** (fork) `rice: notch smoke test and touchpoints`.
