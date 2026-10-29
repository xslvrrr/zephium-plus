import { tabs } from "$domain/tabs";
import { blocker, siteMenuState } from "$domain/blocker";
import { pageSite } from "$domain/site-data";
import { extensions } from "$domain/extensions";
import { webext } from "$domain/webext";
import { commands } from "$shared/ipc/bindings";

// The menu last opened, by whom, and the box an extension popup chosen in it
// hangs from. Each opener holds its own token, so exactly one of them answers
// the extension command the menu sends back.
let opened: { owner: symbol; anchor: DOMRect } | null = null;

/** The sidebar's native menu: the page's controls, site protection, clearing
 *  the site's data, and the extensions' buttons. It hangs from `anchor`. */
export function openSidebarMenu(owner: symbol, anchor: DOMRect) {
  opened = { owner, anchor };
  const active = tabs.activeTab();
  const { siteProtected, canHide } = siteMenuState(blocker.status());
  // A private window keeps nothing to clear, and only a web page has a site.
  const canClearData =
    tabs.profile()?.kind !== "incognito" &&
    (active?.content ?? "web") === "web" &&
    pageSite(active?.url) !== null;
  const listed = extensions
    .activeActions(tabs.profile()?.id ?? null, tabs.activeId())
    .map((action) => ({
      id: action.runtime.install_id,
      label: action.badge ? `${action.label} (${action.badge})` : action.label,
      icon: action.icon_rgba_base64,
      enabled: action.enabled,
    }));
  void commands.sidebarMenuPopup(
    anchor.left,
    anchor.bottom,
    siteProtected,
    canHide,
    canClearData,
    listed,
    webext.isAvailable(),
  );
}

/** Runs the extension a sidebar menu chose, from that menu's anchor, when
 *  `owner` opened it. */
export function runMenuExtension(owner: symbol, id: string) {
  if (opened?.owner !== owner) return;
  const anchor = opened.anchor;
  const profile = tabs.profile()?.id ?? null;
  const tab = tabs.activeId();
  const action = extensions
    .activeActions(profile, tab)
    .find((candidate) => candidate.runtime.install_id === id);
  if (profile === null || tab === null || !action) return;
  void extensions.invoke(profile, tab, action, anchor);
}
