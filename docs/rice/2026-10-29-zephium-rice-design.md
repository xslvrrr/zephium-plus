# Zephium rice fork — design

Date: 2026-10-29. Status: approved in conversation, awaiting written-spec review.

This fork makes Zephium the daily-driver browser of the macOS rice, replacing
Trance (the Zen fork at `~/Downloads/Code/Trance-dev`). It keeps Zephium's
engine, security model and visual system, adopts a chosen subset of Trance's
look, wires it into `rice theme apply`, and lets Claude (signed into the
owner's Claude account through Claude Code or Claude Desktop) drive the
browser over MCP.

There are two independent sub-projects. Each gets its own implementation plan
and can ship alone:

1. **Trance look** — layout, window controls, texture, radius, accent, new tab.
2. **MCP server** — Zephium exposed as an MCP server to local Claude clients.

## 0. Fork setup

- `~/rice/forks/zephium` is its own git repository, not vendored into the rice
  repo. Upstream moves daily, so the fork must be rebaseable. The rice repo
  ignores the directory (`.gitignore`: `forks/zephium/`).
- Remote `upstream` is `https://github.com/zephium-browser/Zephium`. All fork
  work lives on branch `rice`, rebased onto `upstream/main`. No GitHub fork is
  created unless the owner asks for one.
- The bundle identifier stays `app.zephium`, so the existing profile in
  `~/Library/Application Support/app.zephium` carries over.
- Fork builds disable the updater (`tauri-plugin-updater` and the
  `updates.auto-check` path). Otherwise an upstream release would replace the
  fork in `/Applications`. Settings → Updates says the build is a fork and is
  updated by rebuilding.
- Fork-owned code is kept in clearly named files where possible (for example
  `frame/src/styles/axes/rice.css`, `desktop/src/rice_theme.rs`) to keep the
  upstream diff surface small. Every upstream file the fork edits is listed in
  `docs/rice/UPSTREAM-TOUCHPOINTS.md`.

## 1. Trance look

### 1.1 Principles

Zephium's frontend contract (`docs/frontend.md`) holds: customization is token-
and domain-driven, never preference conditionals spread through components.
Every new choice is a validated preference, projected as one `data-*` axis on
the document root, and resolved to tokens in `frame/src/styles/axes/`. Every
choice can be set back to Zephium's own default, so stock Zephium stays
reachable.

From Trance's root-cause analysis (`TRANCE.md` §3): one owner per surface, no
`!important`, no stacked `backdrop-filter`, animate only transform and opacity.

### 1.2 New and extended preferences

Added to `crates/zephium-core/src/preferences.rs` (`KEYS` and
`value_allowed`), with frame defaults in
`frame/src/domain/preferences/preferences.svelte.ts`:

| Key | Values | Default | Axis |
|---|---|---|---|
| `ui.toolbar` | `sidebar`, `top` | `sidebar` | `data-toolbar` |
| `ui.window-controls` | `shown`, `hover` | `shown` | native only |
| `ui.radius` | integer `0`–`16` | Zephium's current token value | `--rice-radius` |
| `ui.texture` | empty or 64-hex blob id | empty | `data-texture` |
| `ui.texture-opacity` | integer `0`–`40` | `15` | `--rice-texture-opacity` |
| `ui.newtab-wordmark` | existing values plus `mark` | unchanged | existing |

Blob ids follow the same rule as `ui.newtab-image`: an admitted local blob,
resolved only inside the current profile's store by the media protocol. Never a
URL or a filesystem path. The production CSP (`script-src 'self'`) is
unchanged.

### 1.3 Top toolbar (`ui.toolbar = top`)

The Trance silhouette: back, forward and reload, a full-width address field and
extension actions run across the top of the content area. The sidebar keeps
tabs, spaces and its foot.

- **Geometry.** `crates/zephium-core/src/layout.rs` gains a toolbar band height
  in `Metrics` (reusing the existing `caption` reservation, which already
  insets content from the top). With the band on and a page present, the chrome
  rect spans the full window width. The content rect is inset below the band
  and to the right of the sidebar. The page view sits above the chrome view, so
  the two never contend for the same pixels outside the content rect. The
  hidden-sidebar shape keeps its current behaviour, with the band added.
- **Frame.** A `Toolbar` component in `frame/src/features/address/` renders the
  band. The address remains exactly one real `HTMLInputElement` with the
  presentation-barrier contract unchanged: one-way `value`, explicit `input`
  handler, no `isTrusted` gating, no drag or responsive duplicates. With
  `data-toolbar="top"` the sidebar's own address field is not rendered, so
  there is never a second address input.
- **Native.** Extension action buttons and their popups move to the band. Their
  anchor rects come from the band, using the same native popup path they use
  today.
- **As built.** The band is drawn only while the sidebar is shown: with the
  sidebar hidden, both the native layout (`layout::sidebar`, chrome width and
  content top) and the frame drop the band, the address returns to the sidebar
  card, and the hidden shape is exactly upstream's. Work also keeps the
  sidebar's field. The band's utilities control opens the sidebar's native
  menu (the one the rail uses: page controls, site protection, clearing site
  data, extension actions), anchored at the band's button; extension popups
  hang from that anchor. The sidebar's DOM utility tray is not used in the
  band, because the band sits over the page's column, where a DOM panel would
  draw under the page view. The rail's own menu stays in top mode; only the
  sidebar header's back, forward and reload give way to the band's.

### 1.4 Window controls (`ui.window-controls = hover`)

As built (an intentional change from the first design, which hid the buttons
with `isHidden` and showed them from a 40 × 40 pt tracking region): in hover
mode the resting discs are not drawn, and the existing AppKit hover fade, the
same one that lights the buttons in shown mode, reveals them while the pointer
is over them. The buttons stay in place and accessible, and are lit before they
can be clicked. No chrome round trip. Fullscreen and the content-fullscreen
path keep their current behaviour.

### 1.5 rice texture, radius and accent

- **Texture.** The rice texture PNG is admitted as a profile blob and painted
  once as a static layer over the chrome surface, at `--rice-texture-opacity`.
  No additional blur, because the rendered texture already carries its blur.
  It never animates and adds no compositor layer beyond one static image.
- **Radius.** `--rice-radius` feeds the existing radius tokens in
  `frame/src/styles/tokens.css` (window card, sidebar card, content frame).
  Native corner masks for the content view follow the same value.
- **Accent.** The rice accent becomes a custom theme: `ui.theme = custom` with
  `ui.theme-custom` seeded from `ramp.accent`, through the existing
  `crates/zephium-core/src/theme.rs` path. No parallel colour system.

### 1.6 New tab mark (`ui.newtab-wordmark = mark`)

Trance's new tab: the owner's wallpaper (existing `ui.newtab-background =
image`), the rice texture, and a single centred mark. Nothing else is drawn —
no clock, greeting or task strip.

- The mark is `wave-light.png` from Trance (`src/zen/trance/images/`), bundled
  in `frame/src/shared/brand/`.
- The holographic treatment is ported from `trance-surfaces.css`: the repeating
  oil-slick sheen ramp, the pointer-driven tilt and parallax, and the glow.
  Transform and opacity only. Under reduced motion the tilt is off and the sheen
  is fixed at its centre position.

### 1.7 rice integration

Preferences live in SQLite (`profile-*.sqlite`), which rice must not write
while Zephium runs. Instead:

- The fork reads `~/Library/Application Support/app.zephium/rice-theme.json` at
  launch and whenever the file changes, using a file watcher like the one notes
  already use for edits from other apps.
- Fields: `accent` (hex), `texture` (absolute PNG path), `texture_opacity`,
  `radius`, `toolbar`, `window_controls`, `newtab_mark`, and the motion
  durations rice owns. Each field is optional; a missing field leaves the
  preference alone.
- `desktop/src/rice_theme.rs` validates each field with the same
  `value_allowed` rules, admits the texture PNG as a blob (bounded size, PNG
  only), and writes the preferences through the normal preference path. That
  path emits the usual change events, so the frame updates live.
- rice-owned values win at every launch and file change, like Trance's
  `user.js`. To hand a setting back to Zephium, remove its field.
- A malformed file is logged and ignored as a whole. Zephium starts normally.
  A field this build does not know is logged and ignored on its own, so a
  newer rice never voids the file.
- As built, the motion-duration fields are out of scope: Zephium has no
  preferences for them. If rice sends them, they are ignored with a log line
  like any other unknown field.
- `"texture": ""` clears `ui.texture`. Settings → Appearance → Toolbar and
  texture sets each rice preference back by hand (toolbar, window controls,
  radius, texture strength, clear texture). An unchanged texture file is not
  imported again: its content digest is compared with the stored `ui.texture`.
- The rice side adds a `[zephium]` target to `~/rice/theme/targets.toml` and a
  `theme/templates/zephium-theme.json.tmpl` template, tier A, requiring
  `/Applications/Zephium.app`.

## 2. MCP server

### 2.1 Goal and boundary

Claude Code and Claude Desktop, signed into the owner's Claude account, can
read and drive the browser. Zephium does not accept Claude subscription
credentials itself. Anthropic's terms do not allow claude.ai login in
third-party products, so the account is used only through Anthropic's own
clients. Zephium's existing Anthropic API-key provider is unchanged.

### 2.2 Reuse

The tools reuse Ask Zephium's existing implementations and admission rules in
`zephium-work-composition/src/page_assist/tools.rs`. These include fencing
untrusted page text (`core::prompt::fence`), refusing sensitive sites and
private windows, and the confirm step for Commit actions. The MCP server is a
second transport onto them, not a second automation path.

### 2.3 Transport

- New crate `crates/zephium-mcp-server`, built on `rmcp = 3.5.0` (already
  pinned) with its `server` feature.
- In the Zephium process, a Unix domain socket at
  `<app data dir>/mcp/mcp.sock` (for the release profile
  `~/Library/Application Support/app.zephium/mcp/mcp.sock`; the dev profile
  uses `app.zephium.dev`). The socket file is mode `0600` inside the `mcp/`
  directory of mode `0700`. The bridge's default path is the release profile's;
  the dev profile needs `ZEPHIUM_MCP_SOCKET` set to its socket. Every accepted connection is
  checked with `getpeereid`; a peer with another uid is closed immediately.
- A stdio bridge binary, `zephium-mcp`, ships in
  `Zephium.app/Contents/MacOS/`. It relays stdin and stdout to the socket and
  nothing else.
- Client configuration:
  - Claude Code: `claude mcp add zephium -- /Applications/Zephium.app/Contents/MacOS/zephium-mcp`
    (bundled via `bundle.externalBin` in `desktop/tauri.macos.conf.json`; the
    macOS `beforeBuildCommand` runs `scripts/rice-build-mcp-bridge.sh`. The
    externalBin file must also exist for any macOS build of `zephium-desktop`
    (`cargo check`, `pnpm dev`): run `sh scripts/rice-build-mcp-bridge.sh`
    once first; macOS CI does so. Under `pnpm dev` the bridge is the one in
    `target/release/`.)
  - Claude Desktop: the same command in `claude_desktop_config.json`.
- If Zephium is not running, the bridge exits with the error "Zephium is not
  running, or Claude access is off." It never launches the browser.

### 2.4 Tools (v1)

| Tool | Result | Gate |
|---|---|---|
| `list_tabs` | id, title, url, space, active | none |
| `read_page(tab)` | bounded reader text (256 items / 48 KiB) and links, fenced as untrusted | sensitive sites and private windows refused, as in Ask |
| `search_history(query, limit)` | title, url, last visit | none |
| `open_tab(url, background)` | new tab id; `http` and `https` only | none |
| `navigate(tab, url)` | `http` and `https` only | none |
| `activate_tab(tab)` | — | none |
| `close_tab(tab)` | — | in-browser confirm, every call |
| `act(tab, step)` | step result | in-browser confirm, every call |

Out of v1: screenshots, restyling, Work hand-off.

### 2.5 Consent and visibility

- Preference `mcp.enabled`, default `false`. When off, no socket exists.
  Settings → Connections shows the switch and the setup command to copy.
- On a client's first connection, a native prompt names it ("Claude Code wants
  to control Zephium") with Allow once, Allow always, and Deny. "Allow always"
  is stored per `clientInfo.name` in the profile store and can be revoked in
  Settings.
- While a client is connected, the sidebar foot shows an indicator with a
  Stop action. Stop turns `mcp.enabled` off.
- The admission prompt is bounded at 120 s; no answer counts as Deny.
- Every tool call writes an audit row (client, tool, tab, time, outcome) to
  `mcp/mcp-audit.log` in the app data dir: JSON lines, mode `0600`, rotated at
  1 MiB to `mcp-audit.log.1`. It is not cleared with browsing data.
- Private windows and their tabs are never listed or reachable; a private tab
  id answers "No such tab".
- Calls in flight are cancelled on Stop and on a client cancel.
- While Claude access is on, the page reader stays on so `read_page` works.

### 2.6 Failure handling

- Each tool call has a 30 s deadline and returns a typed MCP error on timeout,
  refusal or a missing tab.
- If the socket fails to bind because of a stale file, Zephium removes the
  stale socket and retries once. If that fails, Settings → Connections shows
  the error and the switch stays on.

## 3. Verification

Per the owner's global policy, no test suites are run. Verification is:

- `cargo check --workspace` and `cargo clippy` for the changed crates.
- The frame's `svelte-check` and `tsc` typechecks (from `pnpm run check`,
  without its unit-test stage).
- A dev build (`pnpm dev`, profile `app.zephium.dev`), checked by screenshot
  against the Trance reference for the look, and a manual MCP smoke test: run
  `claude mcp add`, then ask Claude Code to list tabs and read the active page.

## 4. Out of scope

- An essentials icon grid and space dots at the sidebar foot (the owner chose
  to keep Zephium's own).
- Rebranding. The product keeps the Zephium name and icon.
- Windows and Linux. This fork targets macOS only, though shared code must still
  compile for the other platforms.
- Routing Claude subscription credentials into Zephium's own Ask or Work.
