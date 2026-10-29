# Site Styles Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give page transparency and site styles their preferences, per-site and per-feature switches in the address field's site menu, a Settings section with update status, rice-theme defaults, and a final size-checked build.

**Architecture:** Four preference keys go through the existing validated store. Per-site and per-feature choices are the `Overrides` file owned by `desktop/src/site_styles.rs`, reached through four new Tauri commands. The site menu gains a lazily loaded section inside the address field's `UtilityTray`; Settings gains a lazily loaded group beside `RiceAppearance`.

**Tech Stack:** Rust (Tauri 2.11 commands, specta), Svelte 5 runes, the frame's `$shared/ui` kit (`Switch`, `Button`, `SettingsGroup`, `SettingsRow`), paraglide messages.

**Spec:** `docs/rice/2026-10-29-notch-and-site-styles-design.md` (§2, §3.2, §3.4, §3.5, §4, §5).

## Global Constraints

- Do not run test suites. Verification: `cargo check` / `cargo clippy --all-targets` for touched crates; `pnpm --dir frame run typecheck`, `lint`, `stylelint`, `format:check`, `knip`, `build`, `budgets`. Unit tests may be written; they are not run.
- **Ruling (bindings):** regenerate `frame/src/shared/ipc/bindings.ts` only with `cargo test -p zephium-desktop --lib export_typescript_bindings`; never hand-edit it.
- Preference keys and values exactly: `ui.page-transparency` `true|false` (default `false`); `site-styles.enabled` `true|false` (default `false`); `site-styles.force` `true|false` (default `false`); `site-styles.auto-update` `true|false` (default `true`). rice-theme defaults: page transparency `true`, site styles `true`.
- Site-styles UI is a lazy chunk; the startup bundle stays within `frame/bundle-budgets.json` and its 16 KiB allowance. Never re-record the budget.
- Exactly one address `<input>` in the chrome document; the site menu lives inside the existing `UtilityTray`, never a second address field.
- The built `.app` must measure under 50 MiB (`du -sm`) and contain `Contents/MacOS/zephium-mcp`.
- Every upstream file edited gets one row in `docs/rice/UPSTREAM-TOUCHPOINTS.md`.
- Commit after each task, message starting `rice:`, trailers:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01ML2gzDQx5BeuAPZFT135zZ
  ```
- Never push. Never touch `~/Library/Application Support/app.zephium` except `rice-theme.json`.

---

### Task 1: Preference keys and rice-theme defaults

**Files:**
- Modify: `crates/zephium-core/src/preferences.rs` (`KEYS`, `value_allowed`)
- Modify: `frame/src/domain/preferences/preferences.svelte.ts` (defaults under `// rice fork`)
- Modify: `desktop/src/rice_theme.rs` (fields `page_transparency: Option<bool>`, `site_styles: Option<bool>`)
- Modify (rice repo): `~/rice/theme/templates/zephium-theme.json.tmpl`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Produces: the four keys of the Global Constraints, readable through `crate::app_setting` and `preferences.value`.

- [ ] **Step 1:** Add the four keys to `KEYS` beside the other rice keys and to `value_allowed`:

```rust
        // rice fork: page transparency and Zen Internet's site styles.
        "ui.page-transparency" | "site-styles.enabled" | "site-styles.force"
        | "site-styles.auto-update" => matches!(value, "true" | "false"),
```

Add to the frame defaults:

```ts
  "ui.page-transparency": "false",
  "site-styles.enabled": "false",
  "site-styles.force": "false",
  "site-styles.auto-update": "true",
```

- [ ] **Step 2:** In `rice_theme.rs`, add the two fields to `RiceTheme` and in `apply`:

```rust
    if let Some(on) = theme.page_transparency {
        set(shell, "ui.page-transparency", on.to_string());
    }
    if let Some(on) = theme.site_styles {
        set(shell, "site-styles.enabled", on.to_string());
    }
```

- [ ] **Step 3:** Add `"page_transparency": true,` and `"site_styles": true,` to `~/rice/theme/templates/zephium-theme.json.tmpl` (valid JSON, commas checked). Commit in `~/rice`: `Zephium template: page transparency and site styles on` with the trailers. **Ruling:** in scope, as in plan 1 Task 11.

- [ ] **Step 4:** Touchpoints rows for `preferences.rs`, `preferences.svelte.ts`.

- [ ] **Step 5: Verify.** `cargo check -p zephium-core -p zephium-desktop`, clippy, frame `typecheck`.

- [ ] **Step 6: Commit** `rice: page transparency and site styles preferences`.

---

### Task 2: Site-styles commands

**Files:**
- Modify: `desktop/src/site_styles.rs` (view and change functions)
- Modify: `desktop/src/lib.rs` (four commands, registered in `specta_builder`)
- Regenerate: `frame/src/shared/ipc/bindings.ts`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `Catalog::site_for`, `normalize_host`, `Overrides`, `site_styles::{update_now, status, save_overrides}` (plan 2).
- Produces (Tauri, all `CallerPolicy::Main`):
  - `site_styles_for(host: String) -> SiteStylesView`
  - `site_styles_set(host: String, change: SiteStylesChange) -> SiteStylesView`
  - `site_styles_status() -> SiteStylesStatus`
  - `site_styles_update_now() -> SiteStylesStatus`

- [ ] **Step 1: Types.** In `site_styles.rs`:

```rust
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FeatureView {
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub on: bool,
    pub transparency: bool,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteStylesView {
    /// The host, normalized; empty when the page has no host.
    pub host: String,
    /// Whether `styles.json` has a style for this host.
    pub styled: bool,
    pub enabled: bool,
    pub forced: bool,
    pub features: Vec<FeatureView>,
}

#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SiteStylesChange {
    Enabled { on: bool },
    Forced { on: bool },
    Feature { key: String, on: bool },
    Reset,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteStylesStatus {
    pub sites: u32,
    /// Milliseconds since the Unix epoch of the last successful update or
    /// unchanged check; `None` while only the bundled snapshot is in use.
    pub updated: Option<f64>,
    pub updating: bool,
    pub error: Option<String>,
}
```

- [ ] **Step 2: Logic.** `view(host)`: normalize; with a matching site, `enabled = override.enabled.unwrap_or(true)`, `forced = false`, each feature's `on` = its override or the default (`!transparency || settings().transparency`). Without one: `styled = false`, `enabled = override.enabled.unwrap_or(true)`, `forced = override.force.unwrap_or(settings().force)`, no features. `change(host, change)`: edit the override under the site key (styled) or the normalized host (unstyled); store only values that differ from the default (remove a value equal to the default; remove the entry when empty); `Reset` removes the entry; `save_overrides`, which refreshes the engine's sheets; return the new view. A host that does not normalize returns an empty view and changes nothing.

- [ ] **Step 3: Commands.** Thin wrappers in `lib.rs` (authorize, then call). `site_styles_update_now` awaits `site_styles::update_now()` and returns `status()` with `error` set from an `Err`. Register all four; regenerate bindings.

- [ ] **Step 4:** Touchpoints rows for `lib.rs`, `bindings.ts`.

- [ ] **Step 5: Verify.** `cargo check -p zephium-desktop`, clippy, frame `typecheck`.

- [ ] **Step 6: Commit** `rice: site styles commands`.

---

### Task 3: The site menu's site-styles section

**Files:**
- Create: `frame/src/features/site-styles/index.ts`
- Create: `frame/src/features/site-styles/components/SiteStyles.svelte` (shell: loads the body on first open, like `SiteDetails.svelte`)
- Create: `frame/src/features/site-styles/components/SiteStylesBody.svelte`
- Modify: `frame/src/app/browser/Shell.svelte` (render `<SiteStyles {page} {open} />` after `<SiteDetails>` in the `addressBlock` tray)
- Modify: `frame/messages/en.json`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `commands.siteStylesFor`, `commands.siteStylesSet`; `SitePage` from `$features/address` (read `address-model.ts` for the page's URL or host field); `preferences.value("site-styles.enabled")`.

- [ ] **Step 1: Shell component.** `SiteStyles.svelte` mirrors `SiteDetails.svelte` exactly: props `{ page: SitePage | null; open: boolean }`; dynamic `import("./SiteStylesBody.svelte")` the first time `open` is true; nothing rendered while `site-styles.enabled` is not `"true"` or the page has no http(s) host.

- [ ] **Step 2: Body.** `SiteStylesBody.svelte`:
  - On mount and whenever the host changes, `commands.siteStylesFor(host)`.
  - A heading row: `m.site_styles_title()` with a `Switch` bound to `view.enabled` (`siteStylesSet(host, { kind: "enabled", on })`).
  - Styled site, enabled: one `Switch` row per feature (`title`, `description` as the row's secondary text), `siteStylesSet(host, { kind: "feature", key, on })`.
  - Unstyled site, enabled: one row `m.site_styles_force()` / `m.site_styles_force_desc()` with a `Switch` for `forced`.
  - A `Reset` text button when any override exists (the view differs from the defaults): `{ kind: "reset" }`.
  - Every change replaces `view` with the command's result. A footnote: `m.site_styles_reload()` ("Applies on reload for this tab") shown after a change while the current page was loaded before it.
  - Style with the menu tokens `SiteDetails.svelte` uses (`--menu-item-inset`, `--text-label`, `--color-muted`).

- [ ] **Step 3: Messages.** Add to `en.json`: `site_styles_title` "Site styles", `site_styles_force` "Make this site transparent", `site_styles_force_desc` "No style exists for this site; clear its background anyway", `site_styles_reset` "Reset", `site_styles_reload` "Reload the page to see every change". Run `pnpm --dir frame run i18n` through `typecheck`.

- [ ] **Step 4:** Touchpoints rows for `Shell.svelte`, `en.json`.

- [ ] **Step 5: Verify.** Frame `typecheck`, `lint`, `stylelint`, `format:check`, `knip`, `build`, `budgets` (the body must be its own chunk).

- [ ] **Step 6: Commit** `rice: site styles in the site menu`.

---

### Task 4: Settings section

**Files:**
- Create: `frame/src/features/settings/components/appearance/SiteStylesGroup.svelte`
- Modify: `frame/src/features/settings/components/sections/AppearancePage.svelte` (render the group after `<RiceAppearance />`)
- Modify: `frame/messages/en.json`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `commands.siteStylesStatus`, `commands.siteStylesUpdateNow`; the four preference keys.

- [ ] **Step 1: Group.** `SiteStylesGroup.svelte`, following `RiceAppearance.svelte`'s structure (`SettingsGroup`, `SettingsRow`, `Switch`, `Button`):
  - Row `appearance.page-transparency`: `m.rice_page_transparency()` / `m.rice_page_transparency_desc()` ("Let the window show through where a page paints nothing. Needs a window material other than Solid."), `Switch` on `ui.page-transparency`; disabled with that description while `ui.material` is `none`.
  - Row `appearance.site-styles`: `m.site_styles_title()` / `m.site_styles_desc()` ("Zen Internet's styles for 600+ sites, by sameerasw. Each site's switches are in its site menu."), `Switch` on `site-styles.enabled`.
  - Row `appearance.site-styles-force`: `m.site_styles_force_all()` / `m.site_styles_force_all_desc()` ("Clear the background on sites without a style"), `Switch` on `site-styles.force`, disabled while site styles are off.
  - Row `appearance.site-styles-update`: `m.site_styles_auto_update()` / status text (`m.site_styles_status({ sites, when })`, or `m.site_styles_status_bundled({ sites })` while `updated` is `null`, or the error), `Switch` on `site-styles.auto-update`, and a `Button` `m.site_styles_update_now()` that awaits `siteStylesUpdateNow()` and shows `updating` while it runs.
  - Load the status on mount; format `when` with `Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" })`.
- [ ] **Step 2:** Lazy-load the group in `AppearancePage.svelte` the way the page already lazy-loads heavy groups (if it does not, a plain import is fine as long as `budgets` passes: Settings is not in the startup graph).
- [ ] **Step 3:** Messages in `en.json` for every new key above.
- [ ] **Step 4:** Touchpoints rows for `AppearancePage.svelte`, `en.json`.
- [ ] **Step 5: Verify.** Frame `typecheck`, `lint`, `stylelint`, `format:check`, `knip`, `build`, `budgets`.
- [ ] **Step 6: Commit** `rice: page transparency and site styles in Settings`.

---

### Task 5: Checklist, as-built notes and the size-checked build

**Files:**
- Modify: `docs/rice/notch-smoke-test.md` (append site-styles checks)
- Modify: `docs/rice/2026-10-29-notch-and-site-styles-design.md` (as-built notes: hook used, spike verdict, anything that differs)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md` (final pass)

- [ ] **Step 1: Checklist.** Append:
  10. Settings → Appearance: page transparency and site styles on.
  11. youtube.com and github.com: the window shows through the page background; text readable.
  12. An unstyled site (for example `https://example.org`): opaque, as before.
  13. YouTube's site menu lists its features with descriptions; turning "Hide filter chips" off and reloading brings the chips back.
  14. On an unstyled site, "Make this site transparent" then reload: transparent. Reset: opaque after reload.
  15. Settings → "Update now": status shows the time; with networking off it shows an error and styles keep working.
  16. Site styles off: every site opaque after reload.

- [ ] **Step 2: Build.** `pnpm -C desktop exec tauri build --bundles app` (the `-no_fixup_chains` flag now comes from `.cargo/config.toml`). Then:

```sh
APP=target/release/bundle/macos/Zephium.app
du -sm "$APP"                      # must print < 50
test -x "$APP/Contents/MacOS/zephium-mcp" && echo mcp-ok
test -f "$APP/Contents/Resources/site-styles/styles.json.gz" && echo styles-ok
test -f "$APP/Contents/Resources/licenses/site-styles/my-internet-MIT.txt" && echo licence-ok
```

If the size is 50 MiB or more, stop and report the largest contributors (`du -sk "$APP"/Contents/*/* | sort -rn | head`) instead of trimming anything on your own.

- [ ] **Step 3:** Record the measured size in the spec's as-built notes.

- [ ] **Step 4: Commit** `rice: site styles checklist and as-built notes`.
