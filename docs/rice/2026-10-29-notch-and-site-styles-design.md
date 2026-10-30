# Zephium rice fork — notch toolbar, page transparency and site styles

Date: 2026-10-29. Branch: `rice`. Supersedes parts of
`docs/rice/2026-10-29-zephium-rice-design.md` §1 (top band, new-tab mark,
mark-only new tab); everything else there stands.

## 0. Summary

The owner's revisions after living with the first build:

1. Replace the Zen/Trance-style top band with a toolbar shaped like
   Zephium's own new-tab notch. The navigation buttons move into the notch.
   On web pages the notch is hidden and slides down when the pointer reaches
   the top edge.
2. Group the hidden window controls (traffic lights) with the hide-sidebar
   button. The group slides down and up instead of fading, with a small
   activation area and a larger keep-up area.
3. Remove the new-tab wallpaper and the rice mark ("it isn't actually Zen").
4. Port webpage transparency and Zen Internet (per-site transparent CSS) into
   the browser.
5. Keep the built `.app` under 50 MiB (46.6 MiB at `cf3722789`).

Owner choices: notch slides over the page (no layout shift); new tab shows
window material + rice texture; site styles bundled + auto-updated; controls
are global, per-site and per-feature.

## 1. Notch toolbar

### 1.1 What goes away

- The 44 px toolbar band: `TOOLBAR_BAND`, `Metrics.toolbar` and the band
  branch in `crates/zephium-core/src/layout.rs`, the band snippet in
  `Shell.svelte`, the band use of `Toolbar.svelte`. In `ui.toolbar=top` the
  page gets the full content rect, exactly as in upstream's shape.
- `frame/src/features/newtab/RiceMark.svelte`, the mark-only new-tab mode,
  the `mark` value of `ui.newtab-wordmark`, `frame/src/shared/brand/wave-light.png`
  (if nothing else uses it), and the `newtab_mark` field in
  `desktop/src/rice_theme.rs` and `~/rice/theme/templates/zephium-theme.json.tmpl`.
- The new-tab wallpaper while `ui.toolbar=top`: no ground image, no notch cut
  into it. Window material and the rice texture (the native backdrop of
  §1.6) show through. Upstream's wallpaper path stays for `ui.toolbar=sidebar`.

The preference key and value stay `ui.toolbar=top` so the rendered
`rice-theme.json` keeps working; Settings labels the value "Notch".

### 1.2 Shape and contents

- A filled shape hanging from the window's top edge, centred over the page
  column (the content rect, not the window), with the flared top corners of
  Zephium's new-tab notch. Geometry reuses `frame/src/features/newtab/lib/ground.ts`
  (`notchShape`, `notchWidth`) and the constants in `newtab/lib/layout.ts`
  (`{ width: 640, height: 50, radius: 18, flare: 14 }`), narrowed to the page
  column when the column is narrower.
- Fill: the chrome surface tint (`--rice-*` / semantic surface tokens), so it
  reads as the same material as the sidebar; `--rice-radius` does not change
  the notch's own geometry.
- Contents, left to right: back, forward, reload; the address field; site
  menu (site details and the site-styles toggles of §3.4); extensions; the
  "…" menu. The address field remains the only address `<input>` in the
  chrome document (presentation-barrier invariant).
- New tab: notch always down; it holds the address field, which is the new
  tab's search. Clock, tasks and the other new-tab blocks keep their
  positions below it.

### 1.3 Reveal over web pages

- Pages take the full content height. The notch rests above the window's top
  edge, out of view, and slides down over the page.
- Activation area: a 4 px strip along the window's top edge, the width of the
  notch's rect.
- Keep-up area: the notch rect grown by 32 px on the left, right and bottom.
  The notch stays down while the pointer is inside it, while the address
  field has focus, and while one of its menus is open.
- Motion: 180 ms ease-out down, the same up. Reduce Motion (Zephium's or the
  system's) makes it instant.
- Keyboard: focusing the address field (Cmd+L) brings the notch down and
  keeps it down until focus leaves.

### 1.4 Window-controls group

- With `ui.window-controls=hover`, the traffic lights and the hide-sidebar
  button form one group at the window's top-left.
- Activation area: about 4 px tall along the top edge, the group's width,
  from the window's left edge.
- Keep-up area: the group's rect grown by 24 px.
- Motion: the native buttons and the web button slide together with the
  timing of §1.3. The native buttons move by animating their frames inside
  the titlebar container; `desktop/src/platform/window_controls.rs` stops
  fading them (`set_bare`'s alpha path is replaced by an offset).
- When the sidebar is shown, the group sits over the sidebar's top; when it
  is hidden, over the page's top-left.
- With `ui.window-controls=shown`, nothing changes from today.

### 1.5 Mechanism

**Chrome over the page with hit regions.** In `ui.toolbar=top` the chrome
webview spans the window beneath the page, as the band layout already does.
To show the notch or the window-controls group over a page, the chrome is
raised over the stage (the existing `raise_chrome` path used by the hidden
sidebar's peek). While raised, the chrome accepts input only inside a list
of rects. Every other point falls through to the page, including clicks and
scrolling.

- Vendored wry: `WryWebView` already has `skips_hit_test` (the whole view
  inert). Add an optional list of input rects; `hitTest:` returns the
  superview's next hit (by returning `None`) for points outside every rect.
  An empty list keeps today's behaviour.
- Engine: `raise_chrome` gains the rect list on macOS (the notch rect, the
  group rect, the hidden sidebar's card when it peeks). The page's cursor
  cover applies only inside the rects.
- The chrome is raised only while something is down, so a transparent
  full-window layer is not composited over the page the rest of the time.

**Region rules.** The frame sends one list of input rects: the notch while
it is down (with any of its open menus), the window-controls group while it
is down, and the hidden sidebar's card while it peeks. The engine raises the
chrome while the list is non-empty or a peek holds it (two flags, raised
when either is set). While raised, a non-empty list limits input to those
rects; an empty list (a peek outside top mode) keeps today's whole-view
input. The page beneath keeps setting its own cursor outside the rects.

**Nothing else paints over the page.** While raised, the chrome document
paints nothing over the content rect: `html[data-chrome-raised]` makes the
body ground transparent, and the rice texture is no longer in the chrome
(§1.6). Page transparency (§2) needs a native material; with
`ui.material=none` it is not applied.

**Pointer zones.** The page view receives mouse events, not the chrome, so
reveal is read natively. `desktop/src/platform/macos/sidebar_edge.rs` already
watches a local `NSEvent` mouse-moved monitor for the left edge; it
generalises into a zone watcher. The frame sends the activation and keep-up
rects for the notch and for the group (command added in the existing
privileged IPC, `CallerPolicy::Main`). Native sends enter and leave events
(`zephium:reach-zone` with the zone name). The left-edge logic in
`edge_reach.rs` is kept as is.

**Alternatives rejected.** A second webview just for the notch duplicates
frame state and IPC authorisation and costs a WKWebView. A native AppKit
notch rebuilds the toolbar outside Svelte.

### 1.6 Native texture backdrop

The rice texture moves from the chrome (`RiceTexture.svelte`, an `<img>`
fixed over the chrome's ground) to one native layer per Main window, above
the window material and beneath both the chrome and the page views. Chrome,
new tab and transparent pages then all sit over the same texture, and
raising the chrome never moves it.

- The layer shows the stored `ui.texture` media (digest) scaled to fill
  (aspect fill), at `ui.texture-opacity`. It is cleared when `ui.texture` is
  empty, the digest is unreadable, or forced colours / Increase Contrast
  apply.
- `RiceTexture.svelte` and its two uses in `Shell.svelte` are removed.
- Over the Solid material (`ui.material=none`) or Reduce Transparency the
  backdrop is cleared: the chrome's opaque ground covers it.
- In `ui.toolbar=sidebar` the new tab keeps upstream's sky ground, without
  the texture.

### 1.7 Window-controls group details

- The traffic lights slide by animating their layers' `transform` (a
  translation), never their frames or `alphaValue`, so Tao's inset and
  AppKit's titlebar layout are untouched (see the module note in
  `window_controls.rs`).
- While the group is up the buttons are hidden (`setHidden`, as
  `set_tucked` already does), so invisible buttons take no clicks.
- The activation area is the top strip of §1.4 together with the buttons'
  own rect, so a pointer that goes straight to the corner still reveals
  them.

## 2. Page transparency

New preference `ui.page-transparency` (`true|false`, upstream default
`false`, rice-theme default `true`).

- Page webviews stop painting WebKit's default background (`drawsBackground`
  off) and set `underPageBackgroundColor` to clear. The window material and
  the rice texture show through wherever a page paints nothing.
- Backstop: a user-level stylesheet `:root { background-color: Canvas; }` on
  every page. A site's own CSS beats a user-level rule, so this fills only
  pages that set no background; unstyled pages stay readable.
- Pages become transparent only where a site style (§3) or forced
  transparency says so, through `!important` author-level rules.
- Zephium's own pages are chrome and already follow the chrome surface.
- Changing the preference applies to new navigations; open pages update on
  reload.

## 3. Site styles (Zen Internet port)

Source: `sameerasw/my-internet`, MIT, "Copyright (c) 2025 Transparent Zen".
`styles.json` at the time of writing: 667 sites, 2070 features, 881 KB raw,
126 KB gzipped. Shape: `{ "website": { "<file>.css": { "<feature>": "<css>" } },
"mapping": { "<file>.css": ["<domain>", …] } }`.

### 3.1 Crate `crates/zephium-site-styles`

Pure Rust, no platform code.

- Parses `styles.json` into sites and features. A feature key is its title,
  optionally followed by ` $ ` and a description. A file's first feature
  named `transparency` (case-insensitive, with or without a site prefix such
  as `yt-`) is the site's transparency feature.
- Matching, from Zen Internet's rules: the file name minus `.css` is the
  domain; `www.` is ignored; a leading `+` also matches subdomains; a leading
  `-` matches the same name under other public suffixes (`-cifraclub.com.br`
  covers `cifraclub.com`); `mapping` points more domains at a file. The most
  specific match wins: exact, then `+`, then `-`, then mapping.
- Produces, for a host and the current settings and overrides, the CSS to
  apply there (or none).
- **No loads from styles.** Updated CSS comes from a third party and is
  applied to every matching page, where a stylesheet could leak page data
  through the requests it makes. A feature whose CSS contains `@import`,
  `@font-face`, `image-set(`, or a `url(` that is not `data:` or a `#`
  fragment is dropped at parse time and counted in the log. (At the time of
  writing `styles.json` has one relative `url()` and no `@import`.)
- A forced-transparency sheet written for this fork (not copied from any
  unlicensed source): `html, body { background: transparent !important; }`
  and a few common root containers.
- Unit tests for parsing and matching are written; per the owner's global
  rule they are not run as part of verification.

### 3.2 Data and updates

- A snapshot of `styles.json` is bundled gzipped as a Tauri resource and read
  with `flate2` (already a dependency).
- The cache in the profile's data directory (`site-styles/styles.json` plus
  its ETag) wins over the snapshot when present and valid.
- Updates: `reqwest` (already a dependency) fetches
  `https://sameerasw.github.io/my-internet/styles.json` at launch and every
  24 h while `site-styles.auto-update` is on, with `If-None-Match`. No
  cookies, no other hosts. The response is capped at 4 MB, parsed and
  validated before it replaces the cache; any failure keeps the last good
  copy and is logged.
- Settings shows the last successful update and an "Update now" button.

### 3.3 Injection

- One `_WKUserStyleSheet` per host, at author level, main frame only, with
  the include pattern `*://<host>/*`, added to a page webview's user content
  controller when that view starts a main-frame navigation to the host (or
  is redirected to it), before the document exists. WebKit applies a sheet
  only to matching documents, so there is no per-page script, no flash
  before the style applies, and site CSP does not block it. Sheets are added
  per host rather than per site because `-domain` entries (any suffix)
  cannot be written as WebKit match patterns; the crate decides the match.
- A change to the global switches or one site's toggles removes the
  fork's sheets from every controller and re-adds them for each view's
  current host.
- The backstop of §2 is installed the same way at user level.
- Private windows get the same sheets; toggles are stored in the profile.
- The private API is allow-listed in the `xtask` boundary checks alongside
  the existing `drawsBackground` use.
- **Spike first.** Plan 2 starts by proving `_WKUserStyleSheet` (creation,
  match patterns, author level, removal) on the shipped WebKit. If it fails,
  the fallback is an isolated-world `WKUserScript` that receives only the
  current site's CSS per navigation and adopts it as a constructed
  stylesheet; that path may flash briefly and is recorded as such.

### 3.4 Controls

Preferences (added to `crates/zephium-core/src/preferences.rs` `KEYS` and
`value_allowed`, and to the frame defaults):

- `site-styles.enabled` `true|false`, upstream default `false`, rice-theme
  default `true`.
- `site-styles.force` `true|false`, default `false`: forced transparency on
  sites with no style.
- `site-styles.auto-update` `true|false`, default `true`.
- Per-site and per-feature overrides live in a profile-stored map (only
  changes from the defaults are stored), not in individual preference keys.

Surfaces:

- The notch's site menu: this site's styles on or off, force on or off, and
  each feature's switch with its description. Defaults follow Zen Internet:
  all features on; the transparency feature follows `ui.page-transparency`.
- Settings, "Site styles" section: the three global switches, last update,
  "Update now". Loaded lazily so the startup bundle budget holds.

### 3.5 Credit

The MIT notice of `sameerasw/my-internet` is added to the bundle's
`licenses/` resources, beside the blocker list notices.

## 4. Build and size

- `.cargo/config.toml` gains `rustflags = ["-C", "link-arg=-Wl,-no_fixup_chains"]`
  for `aarch64-apple-darwin` and `x86_64-apple-darwin`. Without it, release
  builds on this machine's linker (`ld-27037`) with
  `MACOSX_DEPLOYMENT_TARGET=14.0` write proc-macro dylibs that dyld rejects
  ("mis-aligned LINKEDIT string pool"). Recorded in `UPSTREAM-TOUCHPOINTS.md`.
- Size: the built `Zephium.app` must measure under 50 MiB (`du`). The added
  data is about 0.13 MB; removed artwork offsets part of the code growth.
- The frame's startup bundle stays within `frame/bundle-budgets.json`;
  site-style UI is a lazy chunk.

## 5. Verification

Per the owner's global rule, no test suites are run.

- Per task: `cargo check` and `cargo clippy` for touched crates;
  `pnpm --dir frame check` and the frame build; the bundle budget check on
  frame changes; the `xtask` boundary checks when native WebKit calls change.
- At the end: `pnpm -C desktop exec tauri build --bundles app`; the `.app`
  is under 50 MiB and contains `zephium-mcp`.
- Owner checklist, `docs/rice/notch-smoke-test.md`: notch on the new tab;
  reveal over a page and clicks and scrolling outside it reaching the page;
  window-controls group slide; YouTube and GitHub transparent; an unstyled
  site opaque; site menu toggles; "Update now".

## 6. Plans

1. **Notch look**: removals of §1.1, notch component, new-tab integration,
   wry hit regions, pointer zones, window-controls group, rice template.
2. **Page transparency and site-styles core**: §2, crate of §3.1, data and
   updates of §3.2, injection of §3.3 (spike first).
3. **Site-styles controls**: §3.4, §3.5, smoke-test checklist.

Each plan keeps its own ledger. Every upstream file edited is recorded in
`docs/rice/UPSTREAM-TOUCHPOINTS.md`.

## 6.1 As built (plans 2 and 3)

- **Spike verdict** (`docs/rice/spikes/2026-10-29-user-style-sheet.md`):
  `_WKUserStyleSheet` works on macOS 27.2 when created with
  `forWKWebView: nil`; page-scoped sheets do not reach later documents.
- **Hook**: `crates/zephium-engine/src/platform/macos/site_sheets.rs`. Each
  Browse page has a `PageSheets`, told of main-frame navigation attempts
  (wry's `decidePolicyForNavigationAction`, which also sees redirects), the
  engine's `Redirected` and `Committed` transitions, and bound to its view
  after `configure`. It keeps sheets for the committed host and the target
  host only, so the page being left keeps its style until the next commit.
- **No Canvas backstop** (differs from §2): a user-level `:root` colour
  breaks pages that style only `body` (their background stops reaching the
  canvas). Instead, a page view turns `drawsBackground` off and its
  under-page colour clear only while its committed host's style includes
  transparency; every other page paints WebKit's default background as
  before.
- **Live changes** (differs from §2 and §3.3): a changed switch, setting or
  catalog re-resolves every open page's sheets and background at once; no
  reload is needed.
- **Transparency features**: any feature whose title contains
  "transparen" (also one site's "tranparency") follows page transparency,
  which is off over the Solid material.
- **Updates**: redirects are followed only to `sameerasw.github.io`, at most
  three. A failed update keeps the last good catalog and retries hourly.
- **Size**: `Zephium.app` built at the end of plan 3 measures 46.9 MiB
  (`du -sk`: 48024 KiB), with `zephium-mcp`, the styles snapshot and its
  notice inside.
- **Commands**: `site_styles_for`, `site_styles_set`, `site_styles_status`,
  `site_styles_update_now`, all `CallerPolicy::Main`. Overrides live in
  `<data dir>/site-styles/overrides.json` beside the cached `styles.json`.

## 7. Out of scope

- Zen Internet's custom per-site CSS editor beyond feature switches.
- Windows and Linux: new native code stays behind macOS `cfg`s.
- Firefox-only selectors in the styles; WebKit ignores what it cannot parse.
