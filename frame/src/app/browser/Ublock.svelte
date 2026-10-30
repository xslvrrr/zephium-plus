<script lang="ts">
  import { blocker } from "$domain/blocker";
  import { tabs } from "$domain/tabs";
  import { webext } from "$domain/webext";

  /**
   * rice fork: uBlock Origin Lite replaces the built-in blocker. Its install
   * is offered once per profile, through the same review every extension
   * goes through; once it is installed and on, the built-in blocker is
   * turned off for the profile (once, so it can be turned back on) and uBO
   * is given every site, which a blocker needs. Nothing renders here.
   */
  const OFFERED = "rice.ublock-offered:";
  const REPLACED = "rice.ublock-replaced:";

  function seen(key: string): boolean {
    try {
      return localStorage.getItem(key) !== null;
    } catch {
      return true;
    }
  }
  function mark(key: string) {
    try {
      localStorage.setItem(key, "1");
    } catch {
      // Offered again next time.
    }
  }
  function once(key: string): boolean {
    if (seen(key)) return false;
    mark(key);
    return true;
  }

  // Asked once a session, so a failed download is not asked again at once.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- read only inside the effect it guards
  const tried = new Set<string>();
  let profile = $derived(tabs.profile());
  let ublock = $derived(webext.named(webext.UBLOCK_ID));

  // Offered once a profile's extensions are known and it has no uBO; a
  // download that fails is offered again at the next launch.
  $effect(() => {
    const id = profile?.kind === "incognito" ? null : profile?.id;
    if (!id || !webext.isAvailable() || ublock !== null || webext.review() !== null) return;
    if (webext.isInstalling() || tried.has(id) || seen(OFFERED + id)) return;
    tried.add(id);
    void webext.prepareUblock().then(() => {
      if (webext.review()?.id === webext.UBLOCK_ID) mark(OFFERED + id);
    });
  });

  $effect(() => {
    const id = profile?.id;
    if (!id || ublock === null || !ublock.enabled || ublock.profile_id !== id) return;
    if (blocker.status().desired_enabled !== true || blocker.status().protection === "pending")
      return;
    if (!once(REPLACED + id)) return;
    void blocker.setEnabled(false);
    if (ublock.site_scoped && ublock.access !== "all") void webext.setAccess(ublock.id, "all");
  });
</script>
